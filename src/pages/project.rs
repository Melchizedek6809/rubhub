use axum::{
    extract::{Form, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use tower_cookies::Cookies;
use uuid::Uuid;

use crate::{
    app::{self, ProjectSummary},
    entities::{AccessType, project, user},
    services::{
        project::{generate_unique_slug, get_project, parse_public_access, project_access_level},
        repository::{create_bare_repo, get_git_file, get_git_info, get_git_summary},
        session,
        user::get_user_by_name,
        validation::{validate_project_name, validate_uri},
    },
    state::GlobalState,
};

#[derive(Debug, Deserialize)]
pub struct NewProjectForm {
    pub name: String,
    pub public_access: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProjectSettingsForm {
    pub name: String,
    pub description: String,
    pub public_access: String,
    pub main_branch: String,
    pub website: String,
}

async fn not_found() -> (StatusCode, Html<String>) {
    (StatusCode::NOT_FOUND, Html(app::not_found().await))
}

pub async fn projects_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Path(username): Path<String>,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    let Some(owner) = get_user_by_name(&state.db, username).await else {
        return Err(not_found().await);
    };

    let is_owner = session::current_user(&state, &cookies)
        .await
        .map(|user| user.id == owner.id)
        .unwrap_or(false);

    let projects = project::Entity::find()
        .filter(project::Column::Owner.eq(owner.id))
        .all(&state.db)
        .await
        .unwrap_or_default();

    let user = owner.clone();

    let summaries: Vec<_> = projects
        .iter()
        .map(|p| ProjectSummary {
            name: p.name.as_str(),
            slug: p.slug.as_str(),
            owner_name: owner.name.as_str(),
            owner_slug: owner.slug.as_str(),
            description: p.description.as_str(),
        })
        .collect();

    Ok(Html(app::projects(user, &summaries, is_owner).await))
}

pub async fn new_project_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
) -> Result<Html<String>, Redirect> {
    match session::current_user(&state, &cookies).await {
        Ok(_) => Ok(render_new_project_page(&cookies, None, AccessType::Read).await),
        Err(_) => Err(Redirect::to("/login")),
    }
}

pub async fn handle_new_project(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Form(form): Form<NewProjectForm>,
) -> Result<Response, Redirect> {
    let selected_public_access = form
        .public_access
        .as_deref()
        .and_then(|value| parse_public_access(value).ok())
        .unwrap_or(AccessType::Read);

    let current_user = match session::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Err(Redirect::to("/login")),
    };

    let public_access = match form
        .public_access
        .as_deref()
        .map(parse_public_access)
        .unwrap_or(Ok(AccessType::Read))
    {
        Ok(level) => level,
        Err(msg) => {
            return Ok(
                render_new_project_page(&cookies, Some(msg), selected_public_access)
                    .await
                    .into_response(),
            );
        }
    };

    let name = form.name.trim();
    if name.is_empty() {
        return Ok(render_new_project_page(
            &cookies,
            Some("Name is required."),
            selected_public_access,
        )
        .await
        .into_response());
    }

    if let Err(msg) = validate_project_name(name) {
        return Ok(
            render_new_project_page(&cookies, Some(msg), selected_public_access)
                .await
                .into_response(),
        );
    }

    let project_slug = generate_unique_slug(&state, name, current_user.id).await;
    let user_slug = current_user.slug.clone();

    let new_project = project::ActiveModel {
        id: Set(Uuid::new_v4()),
        owner: Set(current_user.id),
        slug: Set(project_slug.clone()),
        name: Set(name.to_owned()),
        description: Set(String::new()),
        public_access: Set(public_access),
        ..Default::default()
    };

    match new_project.insert(&state.db).await {
        Ok(_) => {
            let res = create_bare_repo(&state, user_slug.clone(), project_slug.clone()).await;
            if res.is_err() {
                Ok(render_new_project_page(
                    &cookies,
                    Some("Could not create project."),
                    selected_public_access,
                )
                .await
                .into_response())
            } else {
                Ok(Redirect::to(&format!("/{user_slug}")).into_response())
            }
        }
        Err(_) => Ok(render_new_project_page(
            &cookies,
            Some("Could not create project."),
            selected_public_access,
        )
        .await
        .into_response()),
    }
}

pub async fn project_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Path((username, slug)): Path<(String, String)>,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    let Some(owner) = get_user_by_name(&state.db, username.clone()).await else {
        return Err(not_found().await);
    };

    let Some(project) = get_project(&state.db, owner.id, slug.clone()).await else {
        return Err(not_found().await);
    };

    let Some(summary) = get_git_summary(&state, &username, &slug) else {
        return Err(not_found().await);
    };

    let current = project.main_branch.clone();
    let info = get_git_info(&state, &username, &slug, &current);

    let session_user = session::current_user(&state, &cookies).await.ok();
    let access_level = project_access_level(
        &state,
        session_user.as_ref().map(|user| user.id),
        project.id,
    )
    .await;

    let ssh_clone_url = format!(
        "ssh://git@{}/{}/{}",
        state.config.ssh_public_host, owner.slug, project.slug
    );

    let readme = get_git_file(&state, &username, &slug, &current, "README.md");
    let readme = readme
        .map(|b| {
            let str = String::from_utf8_lossy(&b.data);
            let parser = pulldown_cmark::Parser::new(&str);

            // Write to a new String buffer.
            let mut html_output = String::new();
            pulldown_cmark::html::push_html(&mut html_output, parser);
            html_output
        })
        .ok();

    Ok(Html(
        app::project_with_access(
            owner,
            project,
            access_level,
            ssh_clone_url,
            summary,
            info,
            readme,
        )
        .await,
    ))
}

pub async fn project_settings_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Path((username, slug)): Path<(String, String)>,
) -> Result<Html<String>, Redirect> {
    let Some(owner) = get_user_by_name(&state.db, username.clone()).await else {
        return Err(Redirect::to("/"));
    };

    let Some(project) = get_project(&state.db, owner.id, slug.clone()).await else {
        return Err(Redirect::to(&format!("/{username}")));
    };

    let current_user = match session::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Err(Redirect::to("/login")),
    };

    let access_level = project_access_level(&state, Some(current_user.id), project.id).await;
    if access_level != AccessType::Admin {
        return Err(Redirect::to(&format!("/{username}/{slug}")));
    }

    Ok(render_project_settings_page(&cookies, owner, project, None).await)
}

pub async fn handle_project_settings(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Path((username, slug)): Path<(String, String)>,
    Form(form): Form<ProjectSettingsForm>,
) -> Result<Response, Redirect> {
    let Some(owner) = get_user_by_name(&state.db, username.clone()).await else {
        return Err(Redirect::to("/"));
    };

    let Some(mut project) = get_project(&state.db, owner.id, slug.clone()).await else {
        return Err(Redirect::to(&format!("/{username}")));
    };

    let current_user = match session::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Err(Redirect::to("/login")),
    };

    let access_level = project_access_level(&state, Some(current_user.id), project.id).await;
    if access_level != AccessType::Admin {
        return Err(Redirect::to(&format!("/{username}/{slug}")));
    }

    let name = form.name.trim();
    let description = form.description.trim();
    let main_branch = form.main_branch.trim();
    let website = form.website.trim();

    let public_access = match parse_public_access(&form.public_access) {
        Ok(level) => level,
        Err(msg) => {
            return Ok(
                render_project_settings_page(&cookies, owner, project, Some(msg))
                    .await
                    .into_response(),
            );
        }
    };

    if let Err(msg) = validate_project_name(name) {
        return Ok(
            render_project_settings_page(&cookies, owner, project, Some(msg))
                .await
                .into_response(),
        );
    }

    if !website.is_empty()
        && let Err(msg) = validate_uri(website)
    {
        return Ok(
            render_project_settings_page(&cookies, owner, project, Some(msg))
                .await
                .into_response(),
        );
    }

    if name.is_empty() {
        return Ok(render_project_settings_page(
            &cookies,
            owner,
            project,
            Some("Name is required."),
        )
        .await
        .into_response());
    }
    if main_branch.is_empty() {
        return Ok(render_project_settings_page(
            &cookies,
            owner,
            project,
            Some("Branch name is required."),
        )
        .await
        .into_response());
    }

    project.name = name.to_owned();
    project.public_access = public_access;
    let mut active: project::ActiveModel = project.into();
    active.name = Set(name.to_owned());
    active.public_access = Set(public_access);
    active.description = Set(description.to_owned());
    active.main_branch = Set(main_branch.to_owned());
    active.website = Set(website.to_owned());

    if active.update(&state.db).await.is_err() {
        // A proper error message would be nicer here
        return Err(Redirect::to(&format!("/{username}/{slug}")));
    }

    Ok(Redirect::to(&format!("/{username}/{slug}/settings")).into_response())
}

async fn render_new_project_page(
    _cookies: &tower_cookies::Cookies,
    message: Option<&str>,
    public_access: AccessType,
) -> Html<String> {
    Html(app::new_project(message, public_access).await)
}

async fn render_project_settings_page(
    _cookies: &tower_cookies::Cookies,
    owner: user::Model,
    project: project::Model,
    message: Option<&str>,
) -> Html<String> {
    Html(app::project_settings(owner, project, message).await)
}

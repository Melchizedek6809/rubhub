use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use std::io;
use tokio::{fs, process::Command};
use uuid::Uuid;

use crate::{
    app::{self, ProjectSummary},
    entities::{AccessType, project, user},
    services::{
        csrf, session,
        user::get_user_by_name,
        validation::{slugify, validate_project_name, validate_slug},
    },
    state::GlobalState,
};

#[derive(Debug, Deserialize)]
pub struct NewProjectForm {
    #[serde(rename = "_csrf")]
    pub csrf_token: Option<String>,
    pub name: String,
    pub public_access: Option<String>,
}

pub async fn projects_page(
    state: &GlobalState,
    cookies: tower_cookies::Cookies,
    username: String,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    let Some(owner) = user::Entity::find()
        .filter(user::Column::Name.eq(username.clone()))
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Err(not_found().await);
    };

    let is_owner = session::current_user(state, &cookies)
        .await
        .map(|user| user.id == owner.id)
        .unwrap_or(false);

    let projects = project::Entity::find()
        .filter(project::Column::Owner.eq(owner.id))
        .all(&state.db)
        .await
        .unwrap_or_default();

    let summaries: Vec<_> = projects
        .iter()
        .map(|p| ProjectSummary {
            name: p.name.as_str(),
            slug: p.slug.as_str(),
            owner: owner.name.as_str(),
            description: p.description.as_str(),
        })
        .collect();

    Ok(Html(app::projects(&owner.name, &summaries, is_owner).await))
}

pub async fn new_project_page(
    state: &GlobalState,
    cookies: tower_cookies::Cookies,
) -> Result<Html<String>, Redirect> {
    match session::current_user(state, &cookies).await {
        Ok(_) => Ok(render_new_project_page(&cookies, None, AccessType::Read).await),
        Err(_) => Err(Redirect::to("/login")),
    }
}

pub async fn create_bare_repo(
    state: &GlobalState,
    user: String,
    project: String,
) -> Result<(), std::io::Error> {
    ensure_safe_component(&user)?;
    ensure_safe_component(&project)?;

    let path = state.config.git_root.join(user);
    fs::create_dir_all(&path).await?;

    let path = path.join(project);
    let status = Command::new("git")
        .arg("init")
        .arg("--bare")
        .arg(path)
        .kill_on_drop(true) // makes shutdowns cleaner
        .status()
        .await?;

    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other("git init --bare failed"))
    }
}

pub async fn handle_new_project(
    state: &GlobalState,
    cookies: tower_cookies::Cookies,
    form: NewProjectForm,
) -> Result<Response, Redirect> {
    let selected_public_access = form
        .public_access
        .as_deref()
        .and_then(|value| parse_public_access(value).ok())
        .unwrap_or(AccessType::Read);

    let current_user = match session::current_user(state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Err(Redirect::to("/login")),
    };

    if let Err(err) = csrf::verify_form_token(&cookies, form.csrf_token.as_deref()) {
        return Ok((
            StatusCode::FORBIDDEN,
            render_new_project_page(&cookies, Some(err.message()), selected_public_access).await,
        )
            .into_response());
    }

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

    let slug = generate_unique_slug(state, name, current_user.id).await;
    let username = current_user.name.clone();

    let new_project = project::ActiveModel {
        id: Set(Uuid::new_v4()),
        owner: Set(current_user.id),
        slug: Set(slug.clone()),
        name: Set(name.to_owned()),
        description: Set(String::new()),
        public_access: Set(public_access),
        ..Default::default()
    };

    match new_project.insert(&state.db).await {
        Ok(_) => {
            let res = create_bare_repo(state, username.clone(), slug.clone()).await;
            if res.is_err() {
                Ok(render_new_project_page(
                    &cookies,
                    Some("Could not create project."),
                    selected_public_access,
                )
                .await
                .into_response())
            } else {
                Ok(Redirect::to(&format!("/{username}")).into_response())
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
    state: &GlobalState,
    cookies: tower_cookies::Cookies,
    username: String,
    slug: String,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    let Some(owner) = user::Entity::find()
        .filter(user::Column::Name.eq(username.clone()))
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Err(not_found().await);
    };

    let project = project::Entity::find()
        .filter(project::Column::Owner.eq(owner.id))
        .filter(project::Column::Slug.eq(slug.clone()))
        .one(&state.db)
        .await
        .ok()
        .flatten();

    let Some(project) = project else {
        return Err(not_found().await);
    };

    let session_user = session::current_user(state, &cookies).await.ok();
    let access_level =
        project_access_level(state, session_user.as_ref().map(|user| user.id), project.id).await;
    let can_manage = matches!(access_level, AccessType::Admin);
    let ssh_clone_url = format!(
        "ssh://git@{}/{}/{}",
        state.config.ssh_public_host, owner.name, project.slug
    );

    Ok(Html(
        app::project_with_access(
            &project.name,
            &project.slug,
            &owner.name,
            access_level,
            can_manage,
            ssh_clone_url,
        )
        .await,
    ))
}

pub async fn find_project_by_path(
    state: &GlobalState,
    path: &str,
) -> Option<(project::Model, user::Model)> {
    let trimmed = path.trim_matches('/');
    let mut parts = trimmed.split('/');
    let username = parts.next()?.trim();
    let mut slug = parts.next()?.trim();

    if parts.next().is_some() || username.is_empty() || slug.is_empty() {
        return None;
    }

    if let Some(stripped) = slug.strip_suffix(".git") {
        slug = stripped;
    }

    if validate_slug(username).is_err() || validate_slug(slug).is_err() {
        return None;
    }

    let query = project::Entity::find()
        .find_also_related(user::Entity)
        .filter(project::Column::Slug.eq(slug))
        .filter(user::Column::Name.eq(username));

    let (project, owner) = (query.one(&state.db).await.ok()?)?;

    Some((project, owner?))
}

#[derive(Debug, Deserialize)]
pub struct ProjectSettingsForm {
    #[serde(rename = "_csrf")]
    pub csrf_token: Option<String>,
    pub name: String,
    pub description: String,
    pub public_access: String,
}

pub async fn get_project(
    db: &DatabaseConnection,
    user_id: Uuid,
    project_slug: String,
) -> Option<project::Model> {
    project::Entity::find()
        .filter(project::Column::Owner.eq(user_id))
        .filter(project::Column::Slug.eq(project_slug))
        .one(db)
        .await
        .ok()
        .flatten()
}

pub async fn project_settings_page(
    state: &GlobalState,
    cookies: tower_cookies::Cookies,
    username: String,
    slug: String,
) -> Result<Html<String>, Redirect> {
    let Some(owner) = get_user_by_name(&state.db, username.clone()).await else {
        return Err(Redirect::to("/"));
    };

    let Some(project) = get_project(&state.db, owner.id, slug.clone()).await else {
        return Err(Redirect::to(&format!("/{username}")));
    };

    let current_user = match session::current_user(state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Err(Redirect::to("/login")),
    };

    let access_level = project_access_level(state, Some(current_user.id), project.id).await;
    if access_level != AccessType::Admin {
        return Err(Redirect::to(&format!("/{username}/{slug}")));
    }

    Ok(render_project_settings_page(&cookies, owner, project, None).await)
}

pub async fn handle_project_settings(
    state: &GlobalState,
    cookies: tower_cookies::Cookies,
    username: String,
    slug: String,
    form: ProjectSettingsForm,
) -> Result<Response, Redirect> {
    let Some(owner) = get_user_by_name(&state.db, username.clone()).await else {
        return Err(Redirect::to("/"));
    };

    let Some(mut project) = get_project(&state.db, owner.id, slug.clone()).await else {
        return Err(Redirect::to(&format!("/{username}")));
    };

    let current_user = match session::current_user(state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Err(Redirect::to("/login")),
    };

    let access_level = project_access_level(state, Some(current_user.id), project.id).await;
    if access_level != AccessType::Admin {
        return Err(Redirect::to(&format!("/{username}/{slug}")));
    }

    if let Err(err) = csrf::verify_form_token(&cookies, form.csrf_token.as_deref()) {
        let page =
            render_project_settings_page(&cookies, owner, project, Some(err.message())).await;
        return Ok((StatusCode::FORBIDDEN, page).into_response());
    }

    let name = form.name.trim();
    let description = form.description.trim();
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

    project.name = name.to_owned();
    project.public_access = public_access;
    let mut active: project::ActiveModel = project.into();
    active.name = Set(name.to_owned());
    active.public_access = Set(public_access);
    active.description = Set(description.to_owned());

    if active.update(&state.db).await.is_err() {
        // A proper error message would be nicer here
        return Err(Redirect::to(&format!("/{username}/{slug}")));
    }

    Ok(Redirect::to(&format!("/{username}/{slug}/settings")).into_response())
}

async fn render_new_project_page(
    cookies: &tower_cookies::Cookies,
    message: Option<&str>,
    public_access: AccessType,
) -> Html<String> {
    let csrf_token = csrf::ensure_csrf_cookie(cookies);
    Html(app::new_project(message, &csrf_token, public_access).await)
}

async fn render_project_settings_page(
    cookies: &tower_cookies::Cookies,
    owner: user::Model,
    project: project::Model,
    message: Option<&str>,
) -> Html<String> {
    let csrf_token = csrf::ensure_csrf_cookie(cookies);
    Html(app::project_settings(owner, project, message, &csrf_token).await)
}

async fn not_found() -> (StatusCode, Html<String>) {
    (StatusCode::NOT_FOUND, Html(app::not_found().await))
}

pub async fn project_access_level(
    state: &GlobalState,
    user_id: Option<Uuid>,
    project_id: Uuid,
) -> AccessType {
    let Some(project) = project::Entity::find_by_id(project_id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return AccessType::None;
    };

    project_access_level_for(&project, user_id)
}

fn project_access_level_for(project: &project::Model, user_id: Option<Uuid>) -> AccessType {
    if let Some(uid) = user_id
        && uid == project.owner
    {
        return AccessType::Admin;
    }

    project.public_access
}

fn parse_public_access(value: &str) -> Result<AccessType, &'static str> {
    match value.to_ascii_lowercase().as_str() {
        "none" => Ok(AccessType::None),
        "read" => Ok(AccessType::Read),
        "write" => Ok(AccessType::Write),
        "admin" => Err("Public admin access is not allowed."),
        _ => Err("Invalid access level."),
    }
}

async fn generate_unique_slug(state: &GlobalState, name: &str, owner: Uuid) -> String {
    let base = slugify(name);
    let mut slug = base.clone();

    for _ in 0..5 {
        let exists = project::Entity::find()
            .filter(project::Column::Slug.eq(slug.clone()))
            .one(&state.db)
            .await
            .ok()
            .flatten();

        if exists.is_none() {
            return slug;
        }

        let short = Uuid::new_v4().to_string();
        let short = short.get(..8).unwrap_or(&short);
        slug = format!("{base}-{short}");
    }

    format!("{base}-{}", owner.to_string().get(..8).unwrap_or("project"))
}

fn ensure_safe_component(value: &str) -> io::Result<()> {
    if value.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid path component",
        ));
    }

    if let Err(msg) = validate_slug(value) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, msg));
    }

    Ok(())
}

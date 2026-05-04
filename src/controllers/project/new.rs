use std::sync::Arc;

use askama::Template;
use axum::{
    Form,
    body::Body,
    extract::State,
    http::Response,
    response::{Html, IntoResponse, Redirect},
};
use serde::Deserialize;
use tower_cookies::Cookies;

use time::OffsetDateTime;

use crate::{
    AccessType, GlobalState, Project, User, UserModel,
    models::{ContentPage, RepoEvent},
    services::{
        meta::PageMeta,
        repository::create_bare_repo,
        session,
        validation::{is_reserved_project_name, validate_project_name},
    },
    views::ThemedRender,
};

#[derive(Debug, Deserialize)]
pub struct NewProjectForm {
    pub name: String,
    pub public_access: Option<String>,
}

#[derive(Template)]
#[template(path = "project/new.html")]
struct NewProjectTemplate<'a> {
    message: Option<&'a str>,
    logged_in_user: Option<Arc<User>>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    meta: PageMeta,
}

pub async fn project_new_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
) -> Result<Html<String>, Redirect> {
    let logged_in_user = match session::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Err(Redirect::to("/login")),
    };
    Ok(render_new_project_page(&state, Some(logged_in_user), None).await)
}

async fn render_new_project_page(
    state: &GlobalState,
    logged_in_user: Option<Arc<User>>,
    message: Option<&str>,
) -> Html<String> {
    let sidebar_projects = if let Some(user) = logged_in_user.clone() {
        user.sidebar_projects(state).await
    } else {
        vec![]
    };

    let template = NewProjectTemplate {
        message,
        logged_in_user,
        sidebar_projects,
        content_pages: state.config.content_pages.clone(),
        meta: PageMeta::new("New project - RubHub", "Create a new RubHub project.")
            .robots("noindex,follow"),
    };
    Html(template.render_with_theme())
}

pub async fn project_new_post(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Form(form): Form<NewProjectForm>,
) -> Response<Body> {
    let selected_public_access = form
        .public_access
        .as_deref()
        .and_then(|value| AccessType::parse_public_access(value).ok())
        .unwrap_or(AccessType::Read);

    let current_user = match session::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Redirect::to("/login").into_response(),
    };

    let name = form.name.trim();
    if name.is_empty() {
        return render_new_project_page(&state, Some(current_user), Some("Name is required."))
            .await
            .into_response();
    }

    if let Err(msg) = validate_project_name(name) {
        return render_new_project_page(&state, Some(current_user), Some(msg))
            .await
            .into_response();
    }

    if is_reserved_project_name(name) {
        return render_new_project_page(
            &state,
            Some(current_user),
            Some("That project name is reserved."),
        )
        .await
        .into_response();
    }

    let user_slug = current_user.slug.clone();

    let project = match Project::new(&current_user, name, selected_public_access) {
        Ok(p) => p,
        Err(msg) => {
            return render_new_project_page(&state, Some(current_user), Some(&msg.to_string()))
                .await
                .into_response();
        }
    };

    // Check if repo directory already exists
    let repo_path = state
        .config
        .git_root
        .join(&project.owner)
        .join(&project.slug);
    if tokio::fs::metadata(&repo_path).await.is_ok() {
        return render_new_project_page(&state, Some(current_user), Some("Project already exists"))
            .await
            .into_response();
    }

    // Create bare repo first (metadata is stored within)
    if create_bare_repo(&state, user_slug.clone(), project.slug.clone())
        .await
        .is_err()
    {
        return render_new_project_page(
            &state,
            Some(current_user),
            Some("Could not create project."),
        )
        .await
        .into_response();
    }

    // Emit creation event before saving metadata (which emits branch events)
    state.emit_event(RepoEvent::RepositoryCreated {
        owner: project.owner.clone(),
        project: project.slug.clone(),
        public_access: project.public_access,
        timestamp: OffsetDateTime::now_utc(),
    });

    // Save metadata to auth_store
    if let Err(msg) = project.save(&state).await {
        // Log error but don't fail - project was created, just no metadata yet
        eprintln!("Warning: Could not save project metadata: {}", msg);
    }

    Redirect::to(&project.uri()).into_response()
}

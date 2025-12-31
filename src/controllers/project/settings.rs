use askama::Template;
use axum::{
    Form,
    body::Body,
    extract::State,
    http::Response,
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project, User,
    extractors::PathUserProject,
    models::ContentPage,
    services::{
        session,
        validation::{validate_project_name, validate_uri},
    },
    views::ThemedRender,
};

#[derive(Template)]
#[template(path = "project_settings.html")]
struct ProjectSettingsTemplate<'a> {
    owner: &'a User,
    project: &'a Project,
    access_level: AccessType,
    message: Option<&'a str>,
    logged_in_user: Option<&'a User>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    active_tab: &'static str,
    selected_branch: String,
}

#[derive(Debug, Deserialize)]
pub struct ProjectSettingsForm {
    pub name: String,
    pub description: String,
    pub public_access: String,
    pub main_branch: String,
    pub website: String,
}

pub async fn project_settings_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, project): PathUserProject,
) -> Response<Body> {
    let current_user = match session::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Redirect::to("/login").into_response(),
    };

    let access_level = project.access_level(Some(current_user.slug.clone())).await;
    if access_level != AccessType::Admin {
        return Redirect::to(&project.uri()).into_response();
    }

    render_project_settings_page(&state, &current_user, owner, project, None).await
}

pub async fn project_settings_post(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, mut project): PathUserProject,
    Form(form): Form<ProjectSettingsForm>,
) -> Response<Body> {
    let current_user = match session::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Redirect::to("/login").into_response(),
    };

    let access_level = project.access_level(Some(current_user.slug.clone())).await;
    if access_level != AccessType::Admin {
        return Redirect::to(&project.uri()).into_response();
    }

    let name = form.name.trim();
    let description = form.description.trim();
    let main_branch = form.main_branch.trim();
    let website = form.website.trim();

    let public_access = match AccessType::parse_public_access(&form.public_access) {
        Ok(level) => level,
        Err(msg) => {
            return render_project_settings_page(&state, &current_user, owner, project, Some(msg))
                .await;
        }
    };

    if let Err(msg) = validate_project_name(name) {
        return render_project_settings_page(&state, &current_user, owner, project, Some(msg))
            .await;
    }

    if !website.is_empty()
        && let Err(msg) = validate_uri(website)
    {
        return render_project_settings_page(&state, &current_user, owner, project, Some(msg))
            .await;
    }

    if name.is_empty() {
        return render_project_settings_page(
            &state,
            &current_user,
            owner,
            project,
            Some("Name is required."),
        )
        .await;
    }
    if main_branch.is_empty() {
        return render_project_settings_page(
            &state,
            &current_user,
            owner,
            project,
            Some("Branch name is required."),
        )
        .await;
    }

    project.name = name.to_owned();
    project.public_access = public_access;
    project.description = description.to_owned();
    project.main_branch = main_branch.to_owned();
    project.website = website.to_owned();

    if project
        .save(&state, &current_user.name, &current_user.email)
        .await
        .is_err()
    {
        // A proper error message would be nicer here
        return Redirect::to(&project.uri()).into_response();
    }

    Redirect::to(&project.uri_settings()).into_response()
}

async fn render_project_settings_page(
    state: &GlobalState,
    logged_in_user: &User,
    owner: User,
    project: Project,
    message: Option<&str>,
) -> Response<Body> {
    let sidebar_projects = logged_in_user.sidebar_projects(state).await;
    let template = ProjectSettingsTemplate {
        owner: &owner,
        project: &project,
        access_level: AccessType::Admin, // Only admins can access settings
        message,
        logged_in_user: Some(logged_in_user),
        sidebar_projects,
        content_pages: state.config.content_pages.clone(),
        active_tab: "settings",
        selected_branch: project.main_branch.clone(),
    };
    template.response()
}

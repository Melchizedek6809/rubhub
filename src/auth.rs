use axum::{
    extract::{Form, Path, State},
    http::StatusCode,
    response::{Html, Redirect, Response},
};
use tower_cookies::Cookies;

use crate::{
    services::{
        project::{self, NewProjectForm},
    },
    state::GlobalState,
};

pub async fn projects_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Path(username): Path<String>,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    project::projects_page(&state, cookies, username).await
}

pub async fn new_project_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
) -> Result<Html<String>, Redirect> {
    project::new_project_page(&state, cookies).await
}

pub async fn handle_new_project(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Form(form): Form<NewProjectForm>,
) -> Result<Response, Redirect> {
    project::handle_new_project(&state, cookies, form).await
}

pub async fn project_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Path((username, slug)): Path<(String, String)>,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    project::project_page(&state, cookies, username, slug).await
}

pub async fn project_settings_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Path((username, slug)): Path<(String, String)>,
) -> Result<Html<String>, Redirect> {
    project::project_settings_page(&state, cookies, username, slug).await
}

pub async fn handle_project_settings(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Path((username, slug)): Path<(String, String)>,
    Form(form): Form<project::ProjectSettingsForm>,
) -> Result<Response, Redirect> {
    project::handle_project_settings(&state, cookies, username, slug, form).await
}

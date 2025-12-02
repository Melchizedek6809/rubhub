use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{Html, Redirect, Response},
};
use tower_cookies::Cookies;

use crate::{
    services::{
        project::{self, NewProjectForm},
        session,
        user::{self, LoginForm, SettingsForm},
    },
    state::GlobalState,
};

pub async fn login_page() -> Html<String> {
    user::login_page().await
}

pub async fn handle_login(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Form(form): Form<LoginForm>,
) -> Result<Response, (StatusCode, Html<String>)> {
    user::handle_login(&state, cookies, form).await
}

pub async fn logout(
    State(state): State<GlobalState>,
    cookies: Cookies,
) -> Result<Redirect, (StatusCode, Html<String>)> {
    Ok(session::logout(&state, cookies).await)
}

pub async fn settings_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
) -> Result<Html<String>, Redirect> {
    user::settings_page(&state, cookies).await
}

pub async fn handle_settings(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Form(form): Form<SettingsForm>,
) -> Result<Response, (StatusCode, Html<String>)> {
    user::handle_settings(&state, cookies, form).await
}

pub async fn projects_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
) -> Result<Html<String>, Redirect> {
    project::projects_page(&state, cookies).await
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
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Result<Html<String>, Redirect> {
    project::project_page(&state, cookies, slug).await
}

pub async fn project_settings_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Result<Html<String>, Redirect> {
    project::project_settings_page(&state, cookies, slug).await
}

pub async fn handle_project_settings(
    State(state): State<GlobalState>,
    cookies: Cookies,
    axum::extract::Path(slug): axum::extract::Path<String>,
    Form(form): Form<project::ProjectSettingsForm>,
) -> Result<Response, Redirect> {
    project::handle_project_settings(&state, cookies, slug, form).await
}

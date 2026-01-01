use std::sync::Arc;

use askama::Template;
use axum::{
    Form,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    GlobalState, Project, User, UserModel,
    models::ContentPage,
    services::{
        session as session_service,
        user_profile::find_invalid_ssh_keys,
        validation::validate_username,
    },
    views::ThemedRender,
};

#[derive(Debug, Deserialize)]
pub struct UserSettingsForm {
    pub name: String,
    pub email: String,
    pub default_main_branch: String,
    pub ssh_keys: Option<String>,
}

#[derive(Template)]
#[template(path = "user_settings.html")]
struct UserSettingsTemplate<'a> {
    user: Arc<User>,
    ssh_keys: &'a [String],
    message: Option<&'a str>,
    logged_in_user: Option<Arc<User>>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
}

pub async fn settings_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
) -> Result<Html<String>, Redirect> {
    let current_user = match session_service::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Err(Redirect::to("/login")),
    };
    let ssh_keys = current_user.ssh_keys.clone();

    Ok(render_settings_page(&state, current_user, &ssh_keys, None).await)
}

async fn internal_error(
    state: &GlobalState,
    user: Arc<User>,
    ssh_keys: &[String],
    err: &str,
) -> (StatusCode, Html<String>) {
    eprintln!("auth error: {err}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        render_settings_page(state, user, ssh_keys, Some(err)).await,
    )
}

pub async fn handle_settings(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Form(form): Form<UserSettingsForm>,
) -> Result<Response, (StatusCode, Html<String>)> {
    let current_user = match session_service::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Ok(Redirect::to("/login").into_response()),
    };

    let name = form.name.trim();
    let default_main_branch = form.default_main_branch.trim();
    let email = form.email.trim().to_owned();
    let ssh_keys: Vec<String> = form
        .ssh_keys
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    // Validate SSH keys
    let invalid_keys = find_invalid_ssh_keys(&ssh_keys);
    if !invalid_keys.is_empty() {
        let first_invalid = &invalid_keys[0];
        let preview = if first_invalid.len() > 40 {
            format!("{}...", &first_invalid[..40])
        } else {
            first_invalid.clone()
        };
        return Err((
            StatusCode::BAD_REQUEST,
            render_settings_page(
                &state,
                current_user,
                &ssh_keys,
                Some(&format!("Invalid SSH key format: {}", preview)),
            )
            .await,
        ));
    }

    if default_main_branch.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            render_settings_page(
                &state,
                current_user,
                &ssh_keys,
                Some("Default main branch is required"),
            )
            .await,
        ));
    }

    if let Err(msg) = validate_username(name) {
        return Err((
            StatusCode::BAD_REQUEST,
            render_settings_page(&state, current_user, &ssh_keys, Some(msg)).await,
        ));
    }

    let mut new_user = current_user.as_ref().clone();
    new_user.name = name.to_owned();
    new_user.email = email.to_owned();
    new_user.default_main_branch = default_main_branch.to_owned();
    new_user.ssh_keys = ssh_keys.clone();

    if let Err(err) = new_user.save(&state.auth) {
        return Err(internal_error(&state, current_user, &ssh_keys, &err.to_string()).await);
    }

    Ok(
        render_settings_page(&state, current_user, &ssh_keys, Some("Settings updated."))
            .await
            .into_response(),
    )
}

async fn render_settings_page(
    state: &GlobalState,
    user: Arc<User>,
    ssh_keys: &[String],
    message: Option<&str>,
) -> Html<String> {
    let sidebar_projects = user.sidebar_projects(state).await;

    let template = UserSettingsTemplate {
        user: user.clone(),
        ssh_keys,
        message,
        logged_in_user: Some(user),
        sidebar_projects,
        content_pages: state.config.content_pages.clone(),
    };
    Html(template.render_with_theme())
}

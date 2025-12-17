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
    GlobalState, User,
    services::{
        session as session_service,
        validation::{validate_uri, validate_username},
    },
    views::ThemedRender,
};

#[derive(Debug, Deserialize)]
pub struct UserSettingsForm {
    pub name: String,
    pub email: String,
    pub website: String,
    pub description: String,
    pub default_main_branch: String,
    pub ssh_keys: Option<String>,
}

#[derive(Template)]
#[template(path = "user_settings.html")]
struct UserSettingsTemplate<'a> {
    user: &'a User,
    ssh_keys: &'a [String],
    message: Option<&'a str>,
    logged_in_user: Option<&'a User>,
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

    Ok(render_settings_page(&cookies, current_user, &ssh_keys, None).await)
}

async fn internal_error(
    cookies: &tower_cookies::Cookies,
    user: User,
    ssh_keys: &[String],
    err: &str,
) -> (axum::http::StatusCode, Html<String>) {
    eprintln!("auth error: {err}");
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        render_settings_page(cookies, user, ssh_keys, Some(err)).await,
    )
}

pub async fn handle_settings(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Form(form): Form<UserSettingsForm>,
) -> Result<Response, (axum::http::StatusCode, Html<String>)> {
    let mut current_user = match session_service::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Ok(Redirect::to("/login").into_response()),
    };

    let name = form.name.trim();
    let default_main_branch = form.default_main_branch.trim();
    let email = form.email.trim().to_owned();
    let website = form.website.trim();
    let ssh_keys: Vec<String> = form
        .ssh_keys
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    if default_main_branch.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            render_settings_page(
                &cookies,
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
            render_settings_page(&cookies, current_user, &ssh_keys, Some(msg)).await,
        ));
    }

    if !website.is_empty()
        && let Err(msg) = validate_uri(website)
    {
        return Err((
            StatusCode::BAD_REQUEST,
            render_settings_page(&cookies, current_user, &ssh_keys, Some(msg)).await,
        ));
    }

    current_user.name = name.to_owned();
    current_user.email = email.to_owned();
    current_user.website = website.to_owned();
    current_user.description = form.description.trim().to_owned();
    current_user.default_main_branch = default_main_branch.to_owned();
    current_user.ssh_keys = ssh_keys.clone();

    if let Err(err) = current_user.save(&state).await {
        return Err(internal_error(&cookies, current_user, &ssh_keys, &err.to_string()).await);
    }

    session_service::set_user_cookie(&cookies, current_user.id, &current_user.slug);

    Ok(
        render_settings_page(&cookies, current_user, &ssh_keys, Some("Settings updated."))
            .await
            .into_response(),
    )
}

async fn render_settings_page(
    _cookies: &tower_cookies::Cookies,
    user: User,
    ssh_keys: &[String],
    message: Option<&str>,
) -> Html<String> {
    let template = UserSettingsTemplate {
        user: &user,
        ssh_keys,
        message,
        logged_in_user: Some(&user),
    };
    Html(template.render_with_theme())
}

use askama::Template;
use axum::{
    extract::State,
    response::{Html, Redirect},
};
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    GlobalState, Project, User,
    extractors::CsrfForm,
    models::ContentPage,
    services::{csrf, session},
    views::ThemedRender,
};

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate<'a> {
    message: Option<&'a str>,
    logged_in_user: Option<&'a User>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    csrf_token_field: String,
}

fn render_login_page(message: Option<&str>, csrf_token_field: String) -> Html<String> {
    let template = LoginTemplate {
        message,
        logged_in_user: None,
        sidebar_projects: vec![],
        content_pages: vec![],
        csrf_token_field,
    };
    Html(template.render_with_theme())
}

fn internal_error<E: std::fmt::Display>(
    err: E,
    csrf_token_field: String,
) -> (axum::http::StatusCode, Html<String>) {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        render_login_page(Some(&format!("{err}")), csrf_token_field),
    )
}

pub async fn login_page(State(state): State<GlobalState>, cookies: Cookies) -> Html<String> {
    let token = csrf::get_or_create_token(&state.config.csrf_secret, &cookies);
    let csrf_token_field = csrf::hidden_field(&token);
    render_login_page(None, csrf_token_field)
}

pub async fn handle_login(
    State(state): State<GlobalState>,
    cookies: Cookies,
    CsrfForm(form): CsrfForm<LoginForm>,
) -> Result<Redirect, (axum::http::StatusCode, Html<String>)> {
    let username = form.username.trim();
    let password = form.password.trim();

    handle_login_action(&state, cookies, username, password).await
}

async fn handle_login_action(
    state: &GlobalState,
    cookies: Cookies,
    username: &str,
    password: &str,
) -> Result<Redirect, (axum::http::StatusCode, Html<String>)> {
    let csrf_token_field = csrf::hidden_field(&csrf::get_or_create_token(
        &state.config.csrf_secret,
        &cookies,
    ));

    match User::login(state, username, password).await {
        Ok(user) => {
            if let Err(err) = session::create_session(state, &cookies, user.id, &user.slug).await {
                return Err(internal_error(err, csrf_token_field));
            }
            Ok(Redirect::to(&user.uri()))
        }
        Err(err) => {
            eprintln!("Login failed for '{username}': {err}");
            Err((
                axum::http::StatusCode::UNAUTHORIZED,
                render_login_page(Some("Invalid username or password."), csrf_token_field),
            ))
        }
    }
}

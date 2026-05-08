use std::sync::Arc;

use askama::Template;
use axum::{
    Form,
    extract::State,
    response::{Html, Redirect},
};
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    GlobalState, Project, User,
    models::{ContentPage, UserModel},
    services::{meta::PageMeta, session},
    views::ThemedRender,
};

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

#[derive(Template)]
#[template(path = "auth/login.html")]
struct LoginTemplate<'a> {
    message: Option<&'a str>,
    logged_in_user: Option<Arc<User>>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    meta: PageMeta,
}

fn render_login_page(message: Option<&str>) -> Html<String> {
    let template = LoginTemplate {
        message,
        logged_in_user: None,
        sidebar_projects: vec![],
        content_pages: vec![],
        meta: PageMeta::new("Login - RubHub", "Log in to RubHub.").robots("noindex,follow"),
    };
    Html(template.render_with_theme())
}

fn internal_error<E: std::fmt::Display>(err: E) -> (axum::http::StatusCode, Html<String>) {
    eprintln!("login error: {err}");
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        render_login_page(Some("Something went wrong.")),
    )
}

pub async fn login_page() -> Html<String> {
    render_login_page(None)
}

pub async fn handle_login(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Form(form): Form<LoginForm>,
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
    if let Some(user) = state.auth.login_user(username, password) {
        if let Err(err) = session::create_session(state, &cookies, &user.slug).await {
            return Err(internal_error(err));
        }
        Ok(Redirect::to(&user.uri()))
    } else {
        Err((
            axum::http::StatusCode::UNAUTHORIZED,
            render_login_page(Some("Invalid username or password.")),
        ))
    }
}

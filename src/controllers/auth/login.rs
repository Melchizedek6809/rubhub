use askama::Template;
use axum::{
    Form,
    extract::State,
    response::{Html, Redirect},
};
use serde::Deserialize;

use crate::{GlobalState, User, services::session, views::ThemedRender};

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate<'a> {
    message: Option<&'a str>,
}

async fn render_login_page(message: Option<&str>) -> Html<String> {
    let template = LoginTemplate { message };
    Html(template.render_with_theme())
}

async fn internal_error<E: std::fmt::Display>(err: E) -> (axum::http::StatusCode, Html<String>) {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        render_login_page(Some(&format!("{err}"))).await,
    )
}

pub async fn login_page() -> Html<String> {
    render_login_page(None).await
}

pub async fn handle_login(
    State(state): State<GlobalState>,
    cookies: tower_cookies::Cookies,
    Form(form): Form<LoginForm>,
) -> Result<Redirect, (axum::http::StatusCode, Html<String>)> {
    let username = form.username.trim();
    let password = form.password.trim();

    handle_login_action(&state, cookies, username, password).await
}

async fn handle_login_action(
    state: &GlobalState,
    cookies: tower_cookies::Cookies,
    username: &str,
    password: &str,
) -> Result<Redirect, (axum::http::StatusCode, Html<String>)> {
    match User::login(state, username, password).await {
        Ok(user) => {
            if let Err(err) = session::create_session(state, &cookies, user.id, &user.slug).await {
                return Err(internal_error(err).await);
            }
            Ok(Redirect::to(&user.uri()))
        }
        Err(err) => Err((
            axum::http::StatusCode::UNAUTHORIZED,
            render_login_page(Some(&format!("{err}"))).await,
        )),
    }
}

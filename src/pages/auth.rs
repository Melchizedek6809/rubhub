use axum::{
    Form,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    app,
    entities::user::User,
    services::{
        session,
        validation::{slugify, validate_password, validate_username},
    },
    state::GlobalState,
};

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub action: String,
    pub username: String,
    pub email: String,
    pub password: String,
}

pub async fn logout(
    State(state): State<GlobalState>,
    cookies: Cookies,
) -> Result<Redirect, (StatusCode, Html<String>)> {
    Ok(session::logout(&state, cookies).await)
}

pub async fn login_page() -> Html<String> {
    render_login_page(None).await
}

async fn render_login_page(message: Option<&str>) -> Html<String> {
    Html(app::login(message).await)
}

async fn internal_error<E: std::fmt::Display>(err: E) -> (axum::http::StatusCode, Html<String>) {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        render_login_page(Some(&format!("{err}"))).await,
    )
}

pub async fn handle_login(
    State(state): State<GlobalState>,
    cookies: tower_cookies::Cookies,
    Form(form): Form<LoginForm>,
) -> Result<Response, (axum::http::StatusCode, Html<String>)> {
    let username = form.username.trim();
    let email = form.email.trim();
    let password = form.password.trim();

    let action = form.action.to_lowercase();

    match action.as_str() {
        "login" => handle_login_action(&state, cookies, username, password)
            .await
            .map(IntoResponse::into_response),
        "register" => handle_register_action(&state, cookies, username, email, password)
            .await
            .map(IntoResponse::into_response),
        _ => Err((
            StatusCode::BAD_REQUEST,
            render_login_page(Some("Unsupported action.")).await,
        )),
    }
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

async fn handle_register_action(
    state: &GlobalState,
    cookies: tower_cookies::Cookies,
    username: &str,
    email: &str,
    password: &str,
) -> Result<Redirect, (axum::http::StatusCode, Html<String>)> {
    if let Err(msg) = validate_username(username) {
        return Err((StatusCode::BAD_REQUEST, render_login_page(Some(msg)).await));
    }

    if let Err(msg) = validate_password(password) {
        return Err((StatusCode::BAD_REQUEST, render_login_page(Some(msg)).await));
    }

    let slug = slugify(username);

    let user = User::load(state, &slug).await;
    if user.is_ok() {
        return Err((
            StatusCode::CONFLICT,
            render_login_page(Some("That username is already taken.")).await,
        ));
    };

    match User::new(username, email, password) {
        Ok(user) => match user.save(state).await {
            Ok(_) => {
                if let Err(err) =
                    session::create_session(state, &cookies, user.id, &user.slug).await
                {
                    return Err(internal_error(err).await);
                };
                Ok(Redirect::to(&user.uri()))
            }
            Err(err) => Err(internal_error(err).await),
        },
        Err(err) => Err(internal_error(err).await),
    }
}

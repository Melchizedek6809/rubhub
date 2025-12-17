use askama::Template;
use axum::{
    Form,
    extract::State,
    http::StatusCode,
    response::{Html, Redirect},
};
use serde::Deserialize;

use crate::{
    GlobalState, User,
    services::{
        session,
        validation::{slugify, validate_password, validate_username},
    },
    views::ThemedRender,
};

#[derive(Debug, Deserialize)]
pub struct RegistrationForm {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Template)]
#[template(path = "registration.html")]
struct RegistrationTemplate<'a> {
    message: Option<&'a str>,
    logged_in_user: Option<&'a User>,
}

async fn render_registration_page(message: Option<&str>) -> Html<String> {
    let template = RegistrationTemplate {
        message,
        logged_in_user: None,
    };
    Html(template.render_with_theme())
}

async fn internal_error<E: std::fmt::Display>(err: E) -> (axum::http::StatusCode, Html<String>) {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        render_registration_page(Some(&format!("{err}"))).await,
    )
}

pub async fn registration_page() -> Html<String> {
    render_registration_page(None).await
}

pub async fn handle_registration(
    State(state): State<GlobalState>,
    cookies: tower_cookies::Cookies,
    Form(form): Form<RegistrationForm>,
) -> Result<Redirect, (StatusCode, Html<String>)> {
    let username = form.username.trim();
    let email = form.email.trim();
    let password = form.password.trim();

    handle_registration_action(&state, cookies, username, email, password).await
}

async fn handle_registration_action(
    state: &GlobalState,
    cookies: tower_cookies::Cookies,
    username: &str,
    email: &str,
    password: &str,
) -> Result<Redirect, (StatusCode, Html<String>)> {
    if let Err(msg) = validate_username(username) {
        return Err((
            StatusCode::BAD_REQUEST,
            render_registration_page(Some(msg)).await,
        ));
    }

    if let Err(msg) = validate_password(password) {
        return Err((
            StatusCode::BAD_REQUEST,
            render_registration_page(Some(msg)).await,
        ));
    }

    let slug = slugify(username);

    let user = User::load(state, &slug).await;
    if user.is_ok() {
        return Err((
            StatusCode::CONFLICT,
            render_registration_page(Some("That username is already taken.")).await,
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

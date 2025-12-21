use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, Redirect},
};
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    GlobalState, Project, User,
    extractors::CsrfForm,
    models::ContentPage,
    services::{
        csrf, session,
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
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    csrf_token_field: String,
}

fn render_registration_page(message: Option<&str>, csrf_token_field: String) -> Html<String> {
    let template = RegistrationTemplate {
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
) -> (StatusCode, Html<String>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        render_registration_page(Some(&format!("{err}")), csrf_token_field),
    )
}

pub async fn registration_page(State(state): State<GlobalState>, cookies: Cookies) -> Html<String> {
    let token = csrf::get_or_create_token(&state.config.csrf_secret, &cookies);
    let csrf_token_field = csrf::hidden_field(&token);
    render_registration_page(None, csrf_token_field)
}

pub async fn handle_registration(
    State(state): State<GlobalState>,
    cookies: Cookies,
    CsrfForm(form): CsrfForm<RegistrationForm>,
) -> Result<Redirect, (StatusCode, Html<String>)> {
    let username = form.username.trim();
    let email = form.email.trim();
    let password = form.password.trim();

    handle_registration_action(&state, cookies, username, email, password).await
}

async fn handle_registration_action(
    state: &GlobalState,
    cookies: Cookies,
    username: &str,
    email: &str,
    password: &str,
) -> Result<Redirect, (StatusCode, Html<String>)> {
    let csrf_token_field = csrf::hidden_field(&csrf::get_or_create_token(
        &state.config.csrf_secret,
        &cookies,
    ));

    if let Err(msg) = validate_username(username) {
        return Err((
            StatusCode::BAD_REQUEST,
            render_registration_page(Some(msg), csrf_token_field),
        ));
    }

    if let Err(msg) = validate_password(password) {
        return Err((
            StatusCode::BAD_REQUEST,
            render_registration_page(Some(msg), csrf_token_field),
        ));
    }

    let slug = slugify(username);

    let user = User::load(state, &slug).await;
    if user.is_ok() {
        return Err((
            StatusCode::CONFLICT,
            render_registration_page(Some("That username is already taken."), csrf_token_field),
        ));
    };

    match User::new(username, email, password) {
        Ok(user) => match user.save(state).await {
            Ok(_) => {
                if let Err(err) =
                    session::create_session(state, &cookies, user.id, &user.slug).await
                {
                    return Err(internal_error(err, csrf_token_field));
                };
                Ok(Redirect::to(&user.uri()))
            }
            Err(err) => Err(internal_error(err, csrf_token_field)),
        },
        Err(err) => Err(internal_error(err, csrf_token_field)),
    }
}

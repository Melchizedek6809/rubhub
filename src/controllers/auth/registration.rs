use std::sync::Arc;

use askama::Template;
use axum::{
    Form,
    extract::State,
    http::StatusCode,
    response::{Html, Redirect},
};
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    GlobalState, Project, User, UserModel,
    models::ContentPage,
    services::{
        meta::PageMeta,
        session,
        validation::{slugify, validate_email, validate_password, validate_username},
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
#[template(path = "auth/registration.html")]
struct RegistrationTemplate<'a> {
    message: Option<&'a str>,
    logged_in_user: Option<Arc<User>>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    meta: PageMeta,
}

fn render_registration_page(message: Option<&str>) -> Html<String> {
    let template = RegistrationTemplate {
        message,
        logged_in_user: None,
        sidebar_projects: vec![],
        content_pages: vec![],
        meta: PageMeta::new("Registration - RubHub", "Create a RubHub account.")
            .robots("noindex,follow"),
    };
    Html(template.render_with_theme())
}

fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, Html<String>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        render_registration_page(Some(&format!("{err}"))),
    )
}

pub async fn registration_page() -> Html<String> {
    render_registration_page(None)
}

pub async fn handle_registration(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Form(form): Form<RegistrationForm>,
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
    if let Err(msg) = validate_username(username) {
        return Err((StatusCode::BAD_REQUEST, render_registration_page(Some(msg))));
    }

    if let Err(msg) = validate_email(email) {
        return Err((StatusCode::BAD_REQUEST, render_registration_page(Some(msg))));
    }

    if let Err(msg) = validate_password(password) {
        return Err((StatusCode::BAD_REQUEST, render_registration_page(Some(msg))));
    }

    let slug = slugify(username);

    let user = state.auth.get_user(&slug);
    if user.is_some() {
        return Err((
            StatusCode::CONFLICT,
            render_registration_page(Some("That username is already taken.")),
        ));
    };

    let user = User::new(
        slug.to_string(),
        username.to_string(),
        email.to_string(),
        password.to_string(),
    );
    match user {
        Ok(user) => match user.save(&state.auth) {
            Ok(_) => {
                if let Err(err) = session::create_session(state, &cookies, &slug).await {
                    return Err(internal_error(err));
                };
                let user = state
                    .auth
                    .get_user(&slug)
                    .expect("Couldn't get user after registration");
                Ok(Redirect::to(&user.uri()))
            }
            Err(err) => Err(internal_error(err)),
        },
        Err(err) => Err(internal_error(err)),
    }
}

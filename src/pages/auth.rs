use axum::{
    Form,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use tower_cookies::Cookies;
use uuid::Uuid;

use crate::{
    app,
    entities::{UserType, user},
    services::{
        csrf, session,
        user::{PasswordVerification, hash_password, verify_password_hash},
        validation::{validate_password, validate_username},
    },
    state::GlobalState,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, Condition, EntityTrait, QueryFilter, Set};

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    #[serde(rename = "_csrf")]
    pub csrf_token: Option<String>,
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

pub async fn login_page(cookies: tower_cookies::Cookies) -> Html<String> {
    render_login_page(&cookies, None).await
}

async fn render_login_page(
    cookies: &tower_cookies::Cookies,
    message: Option<&str>,
) -> Html<String> {
    let csrf_token = csrf::ensure_csrf_cookie(cookies);
    Html(app::login(message, &csrf_token).await)
}

async fn internal_error<E: std::fmt::Display>(
    cookies: &tower_cookies::Cookies,
    err: E,
) -> (axum::http::StatusCode, Html<String>) {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        render_login_page(cookies, Some(&format!("{err}"))).await,
    )
}

pub async fn handle_login(
    State(state): State<GlobalState>,
    cookies: tower_cookies::Cookies,
    Form(form): Form<LoginForm>,
) -> Result<Response, (axum::http::StatusCode, Html<String>)> {
    if let Err(err) = csrf::verify_form_token(&cookies, form.csrf_token.as_deref()) {
        return Err((
            StatusCode::FORBIDDEN,
            render_login_page(&cookies, Some(err.message())).await,
        ));
    }

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
            render_login_page(&cookies, Some("Unsupported action.")).await,
        )),
    }
}

async fn handle_login_action(
    state: &GlobalState,
    cookies: tower_cookies::Cookies,
    username: &str,
    password: &str,
) -> Result<Redirect, (axum::http::StatusCode, Html<String>)> {
    let user = match user::Entity::find()
        .filter(user::Column::Name.eq(username))
        .one(&state.db)
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                render_login_page(&cookies, Some("Invalid username or password.")).await,
            ));
        }
        Err(err) => return Err(internal_error(&cookies, err).await),
    };

    let user_id = user.id;
    let stored_hash = user.password_hash.as_deref().unwrap_or("");
    let verification = verify_password_hash(password, stored_hash);
    match verification {
        PasswordVerification::Valid => {}
        PasswordVerification::ValidNeedsRehash { new_hash } => {
            let now = chrono::Utc::now().fixed_offset();
            let mut user_active: user::ActiveModel = user.clone().into();
            user_active.password_hash = Set(Some(new_hash));
            user_active.last_login = Set(Some(now));
            let _ = user_active.update(&state.db).await;
            if let Err(err) = session::create_session(state, &cookies, user_id, username).await {
                return Err(internal_error(&cookies, err).await);
            }
            return Ok(Redirect::to("/"));
        }
        PasswordVerification::Invalid | PasswordVerification::Error => {
            return Err((
                StatusCode::UNAUTHORIZED,
                render_login_page(&cookies, Some("Invalid username or password.")).await,
            ));
        }
    }

    let now = chrono::Utc::now().fixed_offset();
    let mut user_active: user::ActiveModel = user.into();
    user_active.last_login = Set(Some(now));
    let _ = user_active.update(&state.db).await;

    if let Err(err) = session::create_session(state, &cookies, user_id, username).await {
        return Err(internal_error(&cookies, err).await);
    }

    Ok(Redirect::to("/"))
}

async fn handle_register_action(
    state: &GlobalState,
    cookies: tower_cookies::Cookies,
    username: &str,
    email: &str,
    password: &str,
) -> Result<Redirect, (axum::http::StatusCode, Html<String>)> {
    if let Err(msg) = validate_username(username) {
        return Err((
            StatusCode::BAD_REQUEST,
            render_login_page(&cookies, Some(msg)).await,
        ));
    }

    if let Err(msg) = validate_password(password) {
        return Err((
            StatusCode::BAD_REQUEST,
            render_login_page(&cookies, Some(msg)).await,
        ));
    }

    let existing = match user::Entity::find()
        .filter(
            Condition::any()
                .add(user::Column::Name.eq(username))
                .add(user::Column::Email.eq(email)),
        )
        .one(&state.db)
        .await
    {
        Ok(result) => result,
        Err(err) => return Err(internal_error(&cookies, err).await),
    };

    if existing.is_some() {
        return Err((
            StatusCode::CONFLICT,
            render_login_page(&cookies, Some("That username is already taken.")).await,
        ));
    }

    let password_hash = match hash_password(password) {
        Ok(hash) => hash,
        Err(err) => return Err(internal_error(&cookies, err).await),
    };

    let new_user = user::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_type: Set(UserType::Normal),
        name: Set(username.to_owned()),
        email: Set(email.to_owned()),
        password_hash: Set(Some(password_hash)),
        ..Default::default()
    };

    let inserted = match new_user.insert(&state.db).await {
        Ok(user) => user,
        Err(err) => return Err(internal_error(&cookies, err).await),
    };
    if let Err(err) = session::create_session(state, &cookies, inserted.id, username).await {
        return Err(internal_error(&cookies, err).await);
    }

    Ok(Redirect::to("/"))
}

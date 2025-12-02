use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
};
use chrono::{Duration as ChronoDuration, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use time::Duration as CookieDuration;
use tower_cookies::{Cookie, Cookies};
use uuid::Uuid;

use crate::{
    app,
    entities::{UserType, session, user},
    state::GlobalState,
};

const SESSION_COOKIE: &str = "session_id";
const SESSION_USER_COOKIE: &str = "session_user";

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub action: String,
    pub username: String,
    pub password: String,
}

pub async fn login_page() -> Html<String> {
    render_login_page(None).await
}

pub async fn handle_login(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Form(form): Form<LoginForm>,
) -> Result<impl IntoResponse, (StatusCode, Html<String>)> {
    let username = form.username.trim();
    let password = form.password.trim();

    if username.is_empty() || password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            render_login_page(Some("Username and password are required.")).await,
        ));
    }

    let action = form.action.to_lowercase();
    let password_hash = hash_password(password);

    match action.as_str() {
        "login" => handle_login_action(&state, cookies, username, &password_hash).await,
        "register" => handle_register_action(&state, cookies, username, &password_hash).await,
        _ => Err((
            StatusCode::BAD_REQUEST,
            render_login_page(Some("Unsupported action.")).await,
        )),
    }
}

pub async fn logout(
    State(state): State<GlobalState>,
    cookies: Cookies,
) -> Result<Redirect, (StatusCode, Html<String>)> {
    if let Some(existing) = cookies.get(SESSION_COOKIE) {
        if let Ok(session_id) = Uuid::parse_str(existing.value()) {
            let _ = session::Entity::delete_by_id(session_id)
                .exec(&state.db)
                .await;
        }

        cookies.remove(
            Cookie::build((SESSION_COOKIE, ""))
                .path("/")
                .max_age(CookieDuration::seconds(0))
                .build(),
        );
    }

    if cookies.get(SESSION_USER_COOKIE).is_some() {
        cookies.remove(
            Cookie::build((SESSION_USER_COOKIE, ""))
                .path("/")
                .max_age(CookieDuration::seconds(0))
                .build(),
        );
    }

    Ok(Redirect::to("/"))
}

async fn render_login_page(message: Option<&str>) -> Html<String> {
    Html(app::login(message).await)
}

async fn handle_login_action(
    state: &GlobalState,
    cookies: Cookies,
    username: &str,
    password_hash: &str,
) -> Result<Redirect, (StatusCode, Html<String>)> {
    let user = match user::Entity::find()
        .filter(user::Column::Name.eq(username))
        .one(&state.db)
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                render_login_page(Some("Invalid username or password.")).await,
            ));
        }
        Err(err) => return Err(internal_error(err).await),
    };

    let user_id = user.id;
    let stored_hash = user.password_hash.as_deref().unwrap_or("");
    if stored_hash != password_hash {
        return Err((
            StatusCode::UNAUTHORIZED,
            render_login_page(Some("Invalid username or password.")).await,
        ));
    }

    let now = Utc::now().fixed_offset();
    let mut user_active: user::ActiveModel = user.into();
    user_active.last_login = Set(Some(now));
    let _ = user_active.update(&state.db).await;

    create_session(state, &cookies, user_id, username).await?;

    Ok(Redirect::to("/"))
}

async fn handle_register_action(
    state: &GlobalState,
    cookies: Cookies,
    username: &str,
    password_hash: &str,
) -> Result<Redirect, (StatusCode, Html<String>)> {
    let existing = match user::Entity::find()
        .filter(user::Column::Name.eq(username))
        .one(&state.db)
        .await
    {
        Ok(result) => result,
        Err(err) => return Err(internal_error(err).await),
    };

    if existing.is_some() {
        return Err((
            StatusCode::CONFLICT,
            render_login_page(Some("That username is already taken.")).await,
        ));
    }

    let new_user = user::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_type: Set(UserType::Normal),
        name: Set(Some(username.to_owned())),
        password_hash: Set(Some(password_hash.to_owned())),
        ..Default::default()
    };

    let inserted = match new_user.insert(&state.db).await {
        Ok(user) => user,
        Err(err) => return Err(internal_error(err).await),
    };
    create_session(state, &cookies, inserted.id, username).await?;

    Ok(Redirect::to("/"))
}

async fn create_session(
    state: &GlobalState,
    cookies: &Cookies,
    user_id: Uuid,
    username: &str,
) -> Result<(), (StatusCode, Html<String>)> {
    let session_id = Uuid::new_v4();
    let expires_at = Utc::now().fixed_offset() + ChronoDuration::days(30);

    let new_session = session::ActiveModel {
        id: Set(session_id),
        expires_at: Set(Some(expires_at)),
        owner: Set(user_id),
    };

    if let Err(err) = new_session.insert(&state.db).await {
        return Err(internal_error(err).await);
    }

    let cookie = Cookie::build((SESSION_COOKIE, session_id.to_string()))
        .path("/")
        .http_only(true)
        .max_age(CookieDuration::days(30))
        .build();

    cookies.add(cookie);

    let user_info = json!({
        "id": user_id,
        "username": username,
    })
    .to_string();

    let encoded_user_info = urlencoding::encode(&user_info).into_owned();

    let user_cookie = Cookie::build((SESSION_USER_COOKIE, encoded_user_info))
        .path("/")
        .http_only(false)
        .max_age(CookieDuration::days(30))
        .build();

    cookies.add(user_cookie);

    Ok(())
}

fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    format!("{:x}", hasher.finalize())
}

async fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, Html<String>) {
    eprintln!("auth error: {err}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        render_login_page(Some("Something went wrong. Please try again.")).await,
    )
}

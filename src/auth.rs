use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use chrono::{Duration as ChronoDuration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use time::Duration as CookieDuration;
use tower_cookies::{Cookie, Cookies};
use urlencoding;
use uuid::Uuid;

use crate::{
    app,
    entities::{UserType, session, ssh_key, user},
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

#[derive(Debug, Deserialize)]
pub struct SettingsForm {
    pub username: String,
    pub email: Option<String>,
    pub ssh_keys: Option<String>,
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

pub async fn settings_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
) -> Result<Html<String>, Redirect> {
    let current_user = match current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Err(Redirect::to("/login")),
    };

    let keys = ssh_key::Entity::find()
        .filter(ssh_key::Column::UserId.eq(current_user.id))
        .all(&state.db)
        .await
        .unwrap_or_default();

    let ssh_keys: Vec<String> = keys.into_iter().map(|k| k.public_key).collect();

    Ok(Html(
        app::settings(
            current_user.name.as_deref().unwrap_or(""),
            current_user.email.as_deref().unwrap_or(""),
            &ssh_keys,
            None,
        )
        .await,
    ))
}

pub async fn handle_settings(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Form(form): Form<SettingsForm>,
) -> Result<Response, (StatusCode, Html<String>)> {
    let current_user = match current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Ok(Redirect::to("/login").into_response()),
    };

    let username = form.username.trim();
    let email = form.email.unwrap_or_default().trim().to_owned();
    let ssh_keys: Vec<String> = form
        .ssh_keys
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    if username.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Html(app::settings(username, &email, &ssh_keys, Some("Username is required.")).await),
        ));
    }

    let txn = match state.db.begin().await {
        Ok(txn) => txn,
        Err(err) => return Err(internal_error(err).await),
    };

    let mut user_active: user::ActiveModel = current_user.clone().into();
    user_active.name = Set(Some(username.to_owned()));
    user_active.email = Set(if email.is_empty() {
        None
    } else {
        Some(email.clone())
    });

    if let Err(err) = user_active.update(&txn).await {
        return Err(internal_error(err).await);
    }

    if let Err(err) = replace_ssh_keys(&txn, current_user.id, &ssh_keys).await {
        return Err(internal_error(err).await);
    }

    if let Err(err) = txn.commit().await {
        return Err(internal_error(err).await);
    }

    set_user_cookie(&cookies, current_user.id, username);

    Ok(
        Html(app::settings(username, &email, &ssh_keys, Some("Settings updated.")).await)
            .into_response(),
    )
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

    set_user_cookie(cookies, user_id, username);

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

async fn current_user(state: &GlobalState, cookies: &Cookies) -> Result<user::Model, ()> {
    let cookie = cookies.get(SESSION_COOKIE).ok_or(())?;
    let session_id = Uuid::parse_str(cookie.value()).map_err(|_| ())?;

    let session = session::Entity::find_by_id(session_id)
        .one(&state.db)
        .await
        .map_err(|_| ())?
        .ok_or(())?;

    if let Some(expires) = session.expires_at {
        if expires < Utc::now().fixed_offset() {
            return Err(());
        }
    }

    let user = user::Entity::find_by_id(session.owner)
        .one(&state.db)
        .await
        .map_err(|_| ())?
        .ok_or(())?;

    Ok(user)
}

async fn replace_ssh_keys(
    txn: &DatabaseTransaction,
    user_id: Uuid,
    ssh_keys: &[String],
) -> Result<(), sea_orm::DbErr> {
    ssh_key::Entity::delete_many()
        .filter(ssh_key::Column::UserId.eq(user_id))
        .exec(txn)
        .await?;

    if ssh_keys.is_empty() {
        return Ok(());
    }

    let models: Vec<ssh_key::ActiveModel> = ssh_keys
        .iter()
        .map(|key| ssh_key::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            public_key: Set(key.clone()),
            created_at: Set(None),
        })
        .collect();

    ssh_key::Entity::insert_many(models).exec(txn).await?;

    Ok(())
}

fn set_user_cookie(cookies: &Cookies, user_id: Uuid, username: &str) {
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
}

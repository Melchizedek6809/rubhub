use axum::response::{Html, IntoResponse, Redirect, Response};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, EntityTrait, QueryFilter, Set, TransactionTrait
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    app,
    entities::{UserType, ssh_key, user},
    services::session as session_service,
    state::GlobalState,
};

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub action: String,
    pub username: String,
    pub email: String,
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
    state: &GlobalState,
    cookies: tower_cookies::Cookies,
    form: LoginForm,
) -> Result<Response, (axum::http::StatusCode, Html<String>)> {
    let username = form.username.trim();
    let email = form.email.trim();
    let password = form.password.trim();

    if username.is_empty() || password.is_empty() || email.is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            render_login_page(Some("Username and password are required.")).await,
        ));
    }

    let action = form.action.to_lowercase();
    let password_hash = hash_password(password);

    match action.as_str() {
        "login" => handle_login_action(state, cookies, username, &password_hash)
            .await
            .map(IntoResponse::into_response),
        "register" => handle_register_action(state, cookies, username, email, &password_hash)
            .await
            .map(IntoResponse::into_response),
        _ => Err((
            axum::http::StatusCode::BAD_REQUEST,
            render_login_page(Some("Unsupported action.")).await,
        )),
    }
}

pub async fn settings_page(
    state: &GlobalState,
    cookies: tower_cookies::Cookies,
) -> Result<Html<String>, Redirect> {
    let current_user = match session_service::current_user(state, &cookies).await {
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
            &current_user.name,
            &current_user.email,
            &ssh_keys,
            None,
        )
        .await,
    ))
}

pub async fn handle_settings(
    state: &GlobalState,
    cookies: tower_cookies::Cookies,
    form: SettingsForm,
) -> Result<Response, (axum::http::StatusCode, Html<String>)> {
    let current_user = match session_service::current_user(state, &cookies).await {
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
            axum::http::StatusCode::BAD_REQUEST,
            Html(app::settings(username, &email, &ssh_keys, Some("Username is required.")).await),
        ));
    }

    let txn = match state.db.begin().await {
        Ok(txn) => txn,
        Err(err) => return Err(internal_error(err).await),
    };

    let mut user_active: user::ActiveModel = current_user.clone().into();
    user_active.name = Set(username.to_owned());
    user_active.email = Set(email.to_owned());

    if let Err(err) = user_active.update(&txn).await {
        return Err(internal_error(err).await);
    }

    if let Err(err) = replace_ssh_keys(&txn, current_user.id, &ssh_keys).await {
        return Err(internal_error(err).await);
    }

    if let Err(err) = txn.commit().await {
        return Err(internal_error(err).await);
    }

    session_service::set_user_cookie(&cookies, current_user.id, username);

    Ok(
        Html(app::settings(username, &email, &ssh_keys, Some("Settings updated.")).await)
            .into_response(),
    )
}

async fn handle_login_action(
    state: &GlobalState,
    cookies: tower_cookies::Cookies,
    username: &str,
    password_hash: &str,
) -> Result<Redirect, (axum::http::StatusCode, Html<String>)> {
    let user = match user::Entity::find()
        .filter(user::Column::Name.eq(username))
        .one(&state.db)
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            return Err((
                axum::http::StatusCode::UNAUTHORIZED,
                render_login_page(Some("Invalid username or password.")).await,
            ));
        }
        Err(err) => return Err(internal_error(err).await),
    };

    let user_id = user.id;
    let stored_hash = user.password_hash.as_deref().unwrap_or("");
    if stored_hash != password_hash {
        return Err((
            axum::http::StatusCode::UNAUTHORIZED,
            render_login_page(Some("Invalid username or password.")).await,
        ));
    }

    let now = chrono::Utc::now().fixed_offset();
    let mut user_active: user::ActiveModel = user.into();
    user_active.last_login = Set(Some(now));
    let _ = user_active.update(&state.db).await;

    if let Err(err) = session_service::create_session(state, &cookies, user_id, username).await {
        return Err(internal_error(err).await);
    }

    Ok(Redirect::to("/"))
}

async fn handle_register_action(
    state: &GlobalState,
    cookies: tower_cookies::Cookies,
    username: &str,
    email: &str,
    password_hash: &str,
) -> Result<Redirect, (axum::http::StatusCode, Html<String>)> {
    let existing = match user::Entity::find()
        .filter(Condition::any()
            .add(user::Column::Name.eq(username))
            .add(user::Column::Email.eq(email)))
        .one(&state.db)
        .await
    {
        Ok(result) => result,
        Err(err) => return Err(internal_error(err).await),
    };

    if existing.is_some() {
        return Err((
            axum::http::StatusCode::CONFLICT,
            render_login_page(Some("That username is already taken.")).await,
        ));
    }

    let new_user = user::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_type: Set(UserType::Normal),
        name: Set(username.to_owned()),
        email: Set(email.to_owned()),
        password_hash: Set(Some(password_hash.to_owned())),
        ..Default::default()
    };

    let inserted = match new_user.insert(&state.db).await {
        Ok(user) => user,
        Err(err) => return Err(internal_error(err).await),
    };
    if let Err(err) = session_service::create_session(state, &cookies, inserted.id, username).await
    {
        return Err(internal_error(err).await);
    }

    Ok(Redirect::to("/"))
}

fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    format!("{:x}", hasher.finalize())
}

async fn internal_error<E: std::fmt::Display>(err: E) -> (axum::http::StatusCode, Html<String>) {
    eprintln!("auth error: {err}");
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        render_login_page(Some("Something went wrong. Please try again.")).await,
    )
}

async fn render_login_page(message: Option<&str>) -> Html<String> {
    Html(app::login(message).await)
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
            public_key: Set(key.clone()),
            user_id: Set(user_id),
            hostname: Set("".to_owned()),
            created_at: Set(None),
        })
        .collect();

    ssh_key::Entity::insert_many(models).exec(txn).await?;

    Ok(())
}

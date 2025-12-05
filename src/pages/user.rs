use axum::{
    Form,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, QueryFilter, Set, TransactionTrait,
};
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    app,
    entities::{ssh_key, user},
    services::{
        csrf, session as session_service, user::replace_ssh_keys, validation::validate_username,
    },
    state::GlobalState,
};

#[derive(Debug, Deserialize)]
pub struct SettingsForm {
    #[serde(rename = "_csrf")]
    pub csrf_token: Option<String>,
    pub username: String,
    pub email: Option<String>,
    pub ssh_keys: Option<String>,
}

pub async fn settings_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
) -> Result<Html<String>, Redirect> {
    let current_user = match session_service::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Err(Redirect::to("/login")),
    };

    let keys = ssh_key::Entity::find()
        .filter(ssh_key::Column::UserId.eq(current_user.id))
        .all(&state.db)
        .await
        .unwrap_or_default();

    let ssh_keys: Vec<String> = keys
        .into_iter()
        .map(|k| {
            if k.hostname.is_empty() {
                k.public_key
            } else {
                format!("{} {}", k.public_key, k.hostname)
            }
        })
        .collect();

    Ok(render_settings_page(
        &cookies,
        &current_user.name,
        &current_user.email,
        &ssh_keys,
        None,
    )
    .await)
}

async fn internal_error(
    cookies: &tower_cookies::Cookies,
    username: &str,
    email: &str,
    ssh_keys: &[String],
    err: &str,
) -> (axum::http::StatusCode, Html<String>) {
    eprintln!("auth error: {err}");
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        render_settings_page(cookies, username, email, ssh_keys, Some(err)).await,
    )
}

pub async fn handle_settings(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Form(form): Form<SettingsForm>,
) -> Result<Response, (axum::http::StatusCode, Html<String>)> {
    let current_user = match session_service::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Ok(Redirect::to("/login").into_response()),
    };

    if let Err(err) = csrf::verify_form_token(&cookies, form.csrf_token.as_deref()) {
        let ssh_keys: Vec<String> = form
            .ssh_keys
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        return Err((
            StatusCode::FORBIDDEN,
            render_settings_page(
                &cookies,
                current_user.name.as_str(),
                current_user.email.as_str(),
                &ssh_keys,
                Some(err.message()),
            )
            .await,
        ));
    }

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
            render_settings_page(
                &cookies,
                username,
                &email,
                &ssh_keys,
                Some("Username is required."),
            )
            .await,
        ));
    }

    if let Err(msg) = validate_username(username) {
        return Err((
            StatusCode::BAD_REQUEST,
            render_settings_page(&cookies, username, &email, &ssh_keys, Some(msg)).await,
        ));
    }

    if let Ok(Some(_)) = user::Entity::find()
        .filter(
            Condition::all()
                .add(user::Column::Name.eq(username))
                .add(user::Column::Id.ne(current_user.id)),
        )
        .one(&state.db)
        .await
    {
        return Err((
            StatusCode::CONFLICT,
            render_settings_page(
                &cookies,
                username,
                &email,
                &ssh_keys,
                Some("That username is already taken."),
            )
            .await,
        ));
    }

    let txn = match state.db.begin().await {
        Ok(txn) => txn,
        Err(err) => {
            return Err(
                internal_error(&cookies, username, &email, &ssh_keys, &err.to_string()).await,
            );
        }
    };

    let mut user_active: user::ActiveModel = current_user.clone().into();
    user_active.name = Set(username.to_owned());
    user_active.email = Set(email.to_owned());

    if let Err(err) = user_active.update(&txn).await {
        return Err(internal_error(&cookies, username, &email, &ssh_keys, &err.to_string()).await);
    }

    if let Err(err) = replace_ssh_keys(&txn, current_user.id, &ssh_keys).await {
        return Err(internal_error(&cookies, username, &email, &ssh_keys, &err.to_string()).await);
    }

    if let Err(err) = txn.commit().await {
        return Err(internal_error(&cookies, username, &email, &ssh_keys, &err.to_string()).await);
    }

    session_service::set_user_cookie(&cookies, current_user.id, username);

    Ok(render_settings_page(
        &cookies,
        username,
        &email,
        &ssh_keys,
        Some("Settings updated."),
    )
    .await
    .into_response())
}

async fn render_settings_page(
    cookies: &tower_cookies::Cookies,
    username: &str,
    email: &str,
    ssh_keys: &[String],
    message: Option<&str>,
) -> Html<String> {
    let csrf_token = csrf::ensure_csrf_cookie(cookies);
    Html(app::settings(username, email, ssh_keys, message, &csrf_token).await)
}

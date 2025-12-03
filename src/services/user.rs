use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use axum::response::{Html, IntoResponse, Redirect, Response};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    app,
    entities::{UserType, ssh_key, user},
    services::session as session_service,
    state::GlobalState,
};

const USERNAME_BLACKLIST: &[&str] = &[
    "projects", "api", "login", "logout", "settings", "public", "dist", "assets", "news", "blog",
    "about", "tos", "privacy", "forum", "chat",
];

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

    match action.as_str() {
        "login" => handle_login_action(state, cookies, username, password)
            .await
            .map(IntoResponse::into_response),
        "register" => handle_register_action(state, cookies, username, email, password)
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

    Ok(Html(
        app::settings(&current_user.name, &current_user.email, &ssh_keys, None).await,
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

    if let Err(msg) = validate_username(username) {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            Html(app::settings(username, &email, &ssh_keys, Some(msg)).await),
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
            axum::http::StatusCode::CONFLICT,
            Html(
                app::settings(
                    username,
                    &email,
                    &ssh_keys,
                    Some("That username is already taken."),
                )
                .await,
            ),
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
                axum::http::StatusCode::UNAUTHORIZED,
                render_login_page(Some("Invalid username or password.")).await,
            ));
        }
        Err(err) => return Err(internal_error(err).await),
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
            if let Err(err) =
                session_service::create_session(state, &cookies, user_id, username).await
            {
                return Err(internal_error(err).await);
            }
            return Ok(Redirect::to("/"));
        }
        PasswordVerification::Invalid | PasswordVerification::Error => {
            return Err((
                axum::http::StatusCode::UNAUTHORIZED,
                render_login_page(Some("Invalid username or password.")).await,
            ));
        }
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
    password: &str,
) -> Result<Redirect, (axum::http::StatusCode, Html<String>)> {
    if let Err(msg) = validate_username(username) {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            render_login_page(Some(msg)).await,
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
        Err(err) => return Err(internal_error(err).await),
    };

    if existing.is_some() {
        return Err((
            axum::http::StatusCode::CONFLICT,
            render_login_page(Some("That username is already taken.")).await,
        ));
    }

    let password_hash = match hash_password(password) {
        Ok(hash) => hash,
        Err(err) => return Err(internal_error(err).await),
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
        Err(err) => return Err(internal_error(err).await),
    };
    if let Err(err) = session_service::create_session(state, &cookies, inserted.id, username).await
    {
        return Err(internal_error(err).await);
    }

    Ok(Redirect::to("/"))
}

fn validate_username(username: &str) -> Result<(), &'static str> {
    if username.len() < 3 {
        return Err("Username must be at least 3 characters.");
    }

    if !username.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return Err("Username can only contain letters and numbers.");
    }

    let lower = username.to_ascii_lowercase();
    if USERNAME_BLACKLIST.iter().any(|reserved| lower == *reserved) {
        return Err("That username is not allowed.");
    }

    Ok(())
}

fn desired_params() -> Params {
    // 19 MiB memory, 2 iterations, 1 lane keeps CPU modest while resisting GPU attacks.
    Params::new(19 * 1024, 2, 1, None).expect("argon2 params are valid")
}

fn password_hasher() -> Argon2<'static> {
    Argon2::new(Algorithm::Argon2id, Version::V0x13, desired_params())
}

fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    password_hasher()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
}

enum PasswordVerification {
    Valid,
    ValidNeedsRehash { new_hash: String },
    Invalid,
    Error,
}

fn verify_password_hash(password: &str, stored: &str) -> PasswordVerification {
    let parsed = match PasswordHash::new(stored) {
        Ok(hash) => hash,
        Err(_) => return PasswordVerification::Error,
    };

    let hasher = password_hasher();
    if hasher
        .verify_password(password.as_bytes(), &parsed)
        .is_err()
    {
        return PasswordVerification::Invalid;
    }

    // If algorithm, version, or params differ, request a rehash for forward upgrades.
    let needs_rehash = match Params::try_from(&parsed) {
        Ok(params) => {
            let ident = argon2::password_hash::Ident::new("argon2id").unwrap();
            let desired_version: u32 = Version::V0x13.into();

            parsed.algorithm != ident
                || parsed.version.unwrap_or(desired_version) != desired_version
                || params != desired_params()
        }
        Err(_) => true,
    };

    if needs_rehash {
        match hash_password(password) {
            Ok(new_hash) => PasswordVerification::ValidNeedsRehash { new_hash },
            Err(_) => PasswordVerification::Valid, // Do not fail login if rehashing fails
        }
    } else {
        PasswordVerification::Valid
    }
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

    let mut models: Vec<ssh_key::ActiveModel> = Vec::new();
    for raw in ssh_keys {
        if let Some((public_key, hostname)) = parse_ssh_public_key(raw) {
            models.push(ssh_key::ActiveModel {
                public_key: Set(public_key),
                user_id: Set(user_id),
                hostname: Set(hostname.unwrap_or_default()),
                created_at: Set(None),
            });
        }
    }

    ssh_key::Entity::insert_many(models).exec(txn).await?;

    Ok(())
}

fn parse_ssh_public_key(input: &str) -> Option<(String, Option<String>)> {
    let mut parts = input.split_whitespace();
    let algo = parts.next()?;
    let key = parts.next()?;

    let public_key = format!("{algo} {key}");
    let hostname: Option<String> = match parts.next() {
        Some(host) => {
            let mut rest = vec![host.to_owned()];
            rest.extend(parts.map(ToOwned::to_owned));
            Some(rest.join(" "))
        }
        None => None,
    };

    Some((public_key, hostname))
}

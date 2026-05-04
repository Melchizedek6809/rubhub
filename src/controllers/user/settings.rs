use std::sync::Arc;

use askama::Template;
use axum::{
    Form,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use russh::keys::ssh_key;
use serde::Deserialize;
use time::OffsetDateTime;
use tower_cookies::{Cookie, Cookies};

use rubhub_auth_store::{ProjectInfo, Session, SshKey};

use crate::{
    GlobalState, Project, RepoEvent, User, UserModel,
    models::ContentPage,
    services::{meta::PageMeta, session as session_service, validation::validate_username},
    views::ThemedRender,
};

#[derive(Debug, Deserialize)]
pub struct UserSettingsForm {
    pub name: String,
    pub email: String,
    pub default_main_branch: String,
    pub ssh_keys: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UserDeleteForm {
    pub confirmation: String,
}

#[derive(Template)]
#[template(path = "user/settings.html")]
struct UserSettingsTemplate<'a> {
    user: Arc<User>,
    ssh_keys: &'a [String],
    delete_token: String,
    message: Option<&'a str>,
    logged_in_user: Option<Arc<User>>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    meta: PageMeta,
}

/// Validate all SSH keys and return invalid ones
fn find_invalid_ssh_keys(keys: &[String]) -> Vec<String> {
    keys.iter()
        .filter(|key| !key.is_empty() && ssh_key::PublicKey::from_openssh(key).is_err())
        .cloned()
        .collect()
}

fn account_delete_token(user_slug: &str) -> String {
    URL_SAFE_NO_PAD.encode(user_slug)
}

pub async fn settings_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
) -> Result<Html<String>, Redirect> {
    let current_user = match session_service::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Err(Redirect::to("/login")),
    };
    let ssh_keys: Vec<String> = state
        .auth
        .get_ssh_keys_for_user(&current_user.slug)
        .iter()
        .map(|k| k.to_authorized_keys_line())
        .collect();

    Ok(render_settings_page(&state, current_user, &ssh_keys, None).await)
}

async fn internal_error(
    state: &GlobalState,
    user: Arc<User>,
    ssh_keys: &[String],
    err: &str,
) -> (StatusCode, Html<String>) {
    eprintln!("auth error: {err}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        render_settings_page(state, user, ssh_keys, Some(err)).await,
    )
}

pub async fn handle_settings(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Form(form): Form<UserSettingsForm>,
) -> Result<Response, (StatusCode, Html<String>)> {
    let current_user = match session_service::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Ok(Redirect::to("/login").into_response()),
    };

    let name = form.name.trim();
    let default_main_branch = form.default_main_branch.trim();
    let email = form.email.trim().to_owned();
    let ssh_keys_raw: Vec<String> = form
        .ssh_keys
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    // Validate SSH keys
    let invalid_keys = find_invalid_ssh_keys(&ssh_keys_raw);
    if !invalid_keys.is_empty() {
        let first_invalid = &invalid_keys[0];
        let preview = if first_invalid.len() > 40 {
            format!("{}...", &first_invalid[..40])
        } else {
            first_invalid.clone()
        };
        return Err((
            StatusCode::BAD_REQUEST,
            render_settings_page(
                &state,
                current_user,
                &ssh_keys_raw,
                Some(&format!("Invalid SSH key format: {}", preview)),
            )
            .await,
        ));
    }

    if default_main_branch.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            render_settings_page(
                &state,
                current_user,
                &ssh_keys_raw,
                Some("Default main branch is required"),
            )
            .await,
        ));
    }

    if let Err(msg) = validate_username(name) {
        return Err((
            StatusCode::BAD_REQUEST,
            render_settings_page(&state, current_user, &ssh_keys_raw, Some(msg)).await,
        ));
    }

    // Parse submitted keys
    let submitted_keys: Vec<SshKey> = ssh_keys_raw
        .iter()
        .filter_map(|line| SshKey::from_authorized_keys_line(line, current_user.slug.clone()))
        .collect();

    // Get current keys for this user
    let current_keys = state.auth.get_ssh_keys_for_user(&current_user.slug);
    let current_key_data: std::collections::HashSet<&str> =
        current_keys.iter().map(|k| k.public_key.as_str()).collect();
    let submitted_key_data: std::collections::HashSet<&str> = submitted_keys
        .iter()
        .map(|k| k.public_key.as_str())
        .collect();

    // Check for duplicate keys owned by other users
    for key in &submitted_keys {
        if let Some(existing) = state.auth.get_ssh_key(&key.public_key)
            && existing.user_slug != current_user.slug
        {
            return Err((
                StatusCode::BAD_REQUEST,
                render_settings_page(
                    &state,
                    current_user,
                    &ssh_keys_raw,
                    Some("One of the SSH keys is already registered to another user"),
                )
                .await,
            ));
        }
    }

    // Delete removed keys
    for key in &current_keys {
        if !submitted_key_data.contains(key.public_key.as_str())
            && let Err(err) = SshKey::delete(&state.auth, key.public_key.clone())
        {
            return Err(
                internal_error(&state, current_user, &ssh_keys_raw, &err.to_string()).await,
            );
        }
    }

    // Add new keys
    for key in submitted_keys {
        if !current_key_data.contains(key.public_key.as_str())
            && let Err(err) = key.save(&state.auth)
        {
            return Err(
                internal_error(&state, current_user, &ssh_keys_raw, &err.to_string()).await,
            );
        }
    }

    // Update user (without ssh_keys field)
    let mut new_user = current_user.as_ref().clone();
    new_user.name = name.to_owned();
    new_user.email = email.to_owned();
    new_user.default_main_branch = default_main_branch.to_owned();

    if let Err(err) = new_user.save(&state.auth) {
        return Err(internal_error(&state, current_user, &ssh_keys_raw, &err.to_string()).await);
    }

    Ok(render_settings_page(
        &state,
        current_user,
        &ssh_keys_raw,
        Some("Settings updated."),
    )
    .await
    .into_response())
}

pub async fn delete_account(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Path(delete_token): Path<String>,
    Form(form): Form<UserDeleteForm>,
) -> Result<Response, (StatusCode, Html<String>)> {
    let current_user = match session_service::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Ok(Redirect::to("/login").into_response()),
    };

    let ssh_keys: Vec<String> = state
        .auth
        .get_ssh_keys_for_user(&current_user.slug)
        .iter()
        .map(|k| k.to_authorized_keys_line())
        .collect();

    if delete_token != account_delete_token(&current_user.slug)
        || form.confirmation.trim() != current_user.slug
    {
        return Err((
            StatusCode::BAD_REQUEST,
            render_settings_page(
                &state,
                current_user,
                &ssh_keys,
                Some("Confirmation text did not match. Account was not deleted."),
            )
            .await,
        ));
    }

    let project_infos = state.auth.get_projects_for_owner(&current_user.slug);
    let projects: Vec<Project> = project_infos
        .iter()
        .filter_map(|info| Project::from_project_info(info))
        .collect();
    let ssh_keys = state.auth.get_ssh_keys_for_user(&current_user.slug);
    let sessions = state.auth.get_sessions_for_user(&current_user.slug);

    let user_git_path = state.config.git_root.join(&current_user.slug);
    if let Err(err) = tokio::fs::remove_dir_all(&user_git_path).await
        && err.kind() != std::io::ErrorKind::NotFound
    {
        return Err(internal_error(
            &state,
            current_user,
            &[],
            &format!("Failed to delete repositories: {err}"),
        )
        .await);
    }

    for info in project_infos {
        if let Err(err) = ProjectInfo::delete(&state.auth, info.key.clone()) {
            return Err(internal_error(&state, current_user, &[], &err.to_string()).await);
        }
    }

    for key in ssh_keys {
        if let Err(err) = SshKey::delete(&state.auth, key.public_key.clone()) {
            return Err(internal_error(&state, current_user, &[], &err.to_string()).await);
        }
    }

    for session in sessions {
        if let Err(err) = Session::delete(&state.auth, session.session_id) {
            return Err(internal_error(&state, current_user, &[], &err.to_string()).await);
        }
    }

    if let Err(err) = User::delete(&state.auth, current_user.slug.clone()) {
        return Err(internal_error(&state, current_user, &[], &err.to_string()).await);
    }

    for project in projects {
        state.emit_event(RepoEvent::RepositoryDeleted {
            owner: project.owner,
            project: project.slug,
            public_access: project.public_access,
            timestamp: OffsetDateTime::now_utc(),
        });
    }

    cookies.remove(
        Cookie::build((session_service::SESSION_COOKIE, ""))
            .path("/")
            .max_age(time::Duration::seconds(0))
            .build(),
    );

    Ok(Redirect::to("/").into_response())
}

async fn render_settings_page(
    state: &GlobalState,
    user: Arc<User>,
    ssh_keys: &[String],
    message: Option<&str>,
) -> Html<String> {
    let sidebar_projects = user.sidebar_projects(state).await;

    let template = UserSettingsTemplate {
        user: user.clone(),
        ssh_keys,
        delete_token: account_delete_token(&user.slug),
        message,
        logged_in_user: Some(user),
        sidebar_projects,
        content_pages: state.config.content_pages.clone(),
        meta: PageMeta::new("Settings - RubHub", "Account settings on RubHub.")
            .robots("noindex,follow"),
    };
    Html(template.render_with_theme())
}

use axum::{
    Form,
    body::Body,
    extract::State,
    http::Response,
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;
use time::OffsetDateTime;
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project,
    controllers::context::{PageContext, require_user},
    extractors::PathUserProject,
    models::RepoEvent,
    services::{
        project_info::project_link,
        repository::fork_bare_repo,
        validation::{is_reserved_project_name, validate_project_name},
    },
};

#[derive(Debug, Deserialize)]
pub struct ForkProjectForm {
    pub name: String,
    pub public_access: String,
}

pub async fn project_fork_post(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, source_project): PathUserProject,
    Form(form): Form<ForkProjectForm>,
) -> Response<Body> {
    if source_project.public_access == AccessType::None {
        return PageContext::load(&state, &cookies).await.not_found();
    }

    let current_user = match require_user(&state, &cookies).await {
        Ok(user) => user,
        Err(response) => return response,
    };

    let access_level = source_project
        .access_level(Some(current_user.slug.clone()))
        .await;
    if !access_level.is_allowed(AccessType::Read) {
        return Redirect::to(&source_project.uri()).into_response();
    }

    let name = form.name.trim();
    if name.is_empty() || validate_project_name(name).is_err() || is_reserved_project_name(name) {
        return Redirect::to(&source_project.uri()).into_response();
    }

    let public_access = match AccessType::parse_public_access(&form.public_access) {
        Ok(level) => level,
        Err(_) => return Redirect::to(&source_project.uri()).into_response(),
    };

    let mut fork = match Project::new(&current_user, name, public_access) {
        Ok(project) => project,
        Err(_) => return Redirect::to(&source_project.uri()).into_response(),
    };
    fork.description = source_project.description.clone();
    fork.main_branch = source_project.main_branch.clone();
    fork.canonical = source_project
        .canonical
        .clone()
        .or_else(|| Some(project_link(&state, &source_project)));
    fork.upstream = Some(project_link(&state, &source_project));

    let repo_path = state.config.git_root.join(&fork.owner).join(&fork.slug);
    if tokio::fs::metadata(&repo_path).await.is_ok() {
        return Redirect::to(&source_project.uri()).into_response();
    }

    if fork_bare_repo(
        &state,
        &owner.slug,
        &source_project.slug,
        &fork.owner,
        &fork.slug,
    )
    .await
    .is_err()
    {
        let _ = tokio::fs::remove_dir_all(&repo_path).await;
        return Redirect::to(&source_project.uri()).into_response();
    }

    if fork.save(&state).await.is_err() {
        let _ = tokio::fs::remove_dir_all(&repo_path).await;
        return Redirect::to(&source_project.uri()).into_response();
    }

    state.emit_event(RepoEvent::RepositoryCreated {
        owner: fork.owner.clone(),
        project: fork.slug.clone(),
        public_access: fork.public_access,
        timestamp: OffsetDateTime::now_utc(),
    });

    Redirect::to(&fork.uri()).into_response()
}

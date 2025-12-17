use askama::Template;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Html,
};
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project, User,
    controllers::not_found,
    extractors::PathUserProjectBranch,
    services::{
        repository::{GitRefInfo, GitSummary, get_git_info, get_git_summary},
        session,
    },
    views::ThemedRender,
};

#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub page: Option<i32>,
}

#[derive(Template)]
#[template(path = "project_commits.html")]
struct ProjectCommitsTemplate<'a> {
    owner: &'a User,
    project: &'a Project,
    selected_branch: String,
    access_level: AccessType,
    ssh_clone_url: String,
    summary: GitSummary,
    info: Option<GitRefInfo>,
    current_page: i32,
    page_count: i32,
    page_min: i32,
    page_max: i32,
}

pub async fn project_commits_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Query(q): Query<Pagination>,
    PathUserProjectBranch(owner, project, current): PathUserProjectBranch,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    let Some(summary) = get_git_summary(&state, &owner.slug, &project.slug).await else {
        return Err(not_found().await);
    };

    let page_size: i32 = 20;
    let current_page: i32 = q.page.unwrap_or(0);
    let offset = current_page * page_size;
    let info = get_git_info(
        &state,
        &owner.slug,
        &project.slug,
        &current,
        page_size,
        offset,
    )
    .await;
    let commit_count = info.as_ref().map(|i| i.commit_count).unwrap_or(0);
    let page_count = commit_count / page_size;

    let session_user = session::current_user(&state, &cookies).await.ok();
    let access_level = project
        .access_level(session_user.as_ref().map(|user| user.slug.clone()))
        .await;
    let git_user = session_user.map(|u| u.slug).unwrap_or("anon".to_string());

    let ssh_clone_url = format!(
        "ssh://{}@{}/{}/{}",
        git_user, state.config.ssh_public_host, owner.slug, project.slug
    );

    let selected_branch = info
        .as_ref()
        .map(|i| i.branch_name.to_string())
        .unwrap_or_default();

    let page_min: i32 = (current_page - 5).max(0);
    let page_max: i32 = (current_page + 5).min(page_count);

    let template = ProjectCommitsTemplate {
        owner: &owner,
        project: &project,
        access_level,
        ssh_clone_url,
        summary,
        info,
        selected_branch,
        current_page,
        page_count,
        page_min,
        page_max,
    };
    Ok(Html(template.render_with_theme()))
}

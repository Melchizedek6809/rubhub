use std::sync::Arc;

use askama::Template;
use axum::{
    body::Body,
    extract::{Query, State},
    http::Response,
};
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project, User,
    controllers::context::ProjectRepoContext,
    extractors::PathUserProjectBranch,
    models::{ContentPage, user::UserModel},
    services::repository::{GitRefInfo, GitSummary, get_git_info},
    views::ThemedRender,
};

#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub page: Option<i32>,
}

#[derive(Template)]
#[template(path = "project/commits.html")]
struct ProjectCommitsTemplate<'a> {
    owner: Arc<User>,
    project: &'a Project,
    selected_branch: String,
    ssh_clone_url: String,
    http_clone_url: String,
    summary: GitSummary,
    info: Option<GitRefInfo>,
    current_page: i32,
    page_count: i32,
    page_min: i32,
    page_max: i32,
    logged_in_user: Option<Arc<User>>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    access_level: AccessType,
    active_tab: &'static str,
}

pub async fn project_commits_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Query(q): Query<Pagination>,
    PathUserProjectBranch(owner, project, current): PathUserProjectBranch,
) -> Response<Body> {
    let repo_context = match ProjectRepoContext::load(&state, &cookies, &owner.slug, &project).await
    {
        Ok(context) => context,
        Err(response) => return response,
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

    let selected_branch = info
        .as_ref()
        .map(|i| i.branch_name.to_string())
        .unwrap_or_default();

    let page_min: i32 = (current_page - 5).max(0);
    let page_max: i32 = (current_page + 5).min(page_count);

    let template = ProjectCommitsTemplate {
        owner,
        project: &project,
        ssh_clone_url: repo_context.ssh_clone_url,
        http_clone_url: repo_context.http_clone_url,
        summary: repo_context.summary,
        info,
        selected_branch,
        current_page,
        page_count,
        page_min,
        page_max,
        access_level: repo_context.project.access_level,
        active_tab: "",
        logged_in_user: repo_context.project.page.logged_in_user,
        sidebar_projects: repo_context.project.page.sidebar_projects,
        content_pages: repo_context.project.page.content_pages,
    };
    template.response()
}

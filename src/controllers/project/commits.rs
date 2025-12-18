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
    controllers::not_found,
    extractors::PathUserProjectBranch,
    models::ContentPage,
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
    http_clone_url: String,
    summary: GitSummary,
    info: Option<GitRefInfo>,
    current_page: i32,
    page_count: i32,
    page_min: i32,
    page_max: i32,
    logged_in_user: Option<&'a User>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
}

pub async fn project_commits_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Query(q): Query<Pagination>,
    PathUserProjectBranch(owner, project, current): PathUserProjectBranch,
) -> Response<Body> {
    let logged_in_user = session::current_user(&state, &cookies).await.ok();

    let sidebar_projects = if let Some(ref user) = logged_in_user {
        user.sidebar_projects(&state).await
    } else {
        vec![]
    };

    let access_level = project
        .access_level(logged_in_user.as_ref().map(|user| user.slug.clone()))
        .await;

    if access_level == AccessType::None {
        return not_found(logged_in_user, vec![]);
    }
    let Some(summary) = get_git_summary(&state, &owner.slug, &project.slug).await else {
        return not_found(logged_in_user, vec![]);
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

    let git_user = logged_in_user
        .as_ref()
        .map(|u| u.slug.clone())
        .unwrap_or("anon".to_string());

    let ssh_clone_url = project.ssh_clone_url(&state.config.ssh_public_host, &git_user);
    let http_clone_url = project.http_clone_url(&state.config.base_url);

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
        http_clone_url,
        summary,
        info,
        selected_branch,
        current_page,
        page_count,
        page_min,
        page_max,
        logged_in_user: logged_in_user.as_ref(),
        sidebar_projects,
        content_pages: state.config.content_pages.clone(),
    };
    template.response()
}

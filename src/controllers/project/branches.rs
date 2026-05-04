use std::sync::Arc;

use askama::Template;
use axum::{body::Body, extract::State, response::Response};
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project, User,
    controllers::context::ProjectRepoContext,
    extractors::PathUserProject,
    models::{ContentPage, user::UserModel},
    services::repository::{GitRefInfo, get_git_info},
    views::ThemedRender,
};

#[derive(Template)]
#[template(path = "project/branches.html")]
struct ProjectBranchesTemplate<'a> {
    owner: Arc<User>,
    project: &'a Project,
    access_level: AccessType,
    selected_branch: &'a str,
    branches: Vec<GitRefInfo>,
    logged_in_user: Option<Arc<User>>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    active_tab: &'static str,
}

pub async fn project_branches_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, project): PathUserProject,
) -> Response<Body> {
    let repo_context = match ProjectRepoContext::load(&state, &cookies, &owner.slug, &project).await
    {
        Ok(context) => context,
        Err(response) => return response,
    };

    let mut branches: Vec<GitRefInfo> = vec![];
    for b in repo_context.summary.branches() {
        if let Some(info) = get_git_info(&state, &owner.slug, &project.slug, b, 1, 0).await {
            branches.push(info);
        }
    }

    let template = ProjectBranchesTemplate {
        owner,
        project: &project,
        access_level: repo_context.project.access_level,
        selected_branch: &project.main_branch,
        branches,
        logged_in_user: repo_context.project.page.logged_in_user,
        sidebar_projects: repo_context.project.page.sidebar_projects,
        content_pages: repo_context.project.page.content_pages,
        active_tab: "",
    };
    template.response()
}

use std::sync::Arc;

use askama::Template;
use axum::{body::Body, extract::State, http::Response};
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project, User, UserModel,
    controllers::context::ProjectRepoContext,
    extractors::PathUserProject,
    models::ContentPage,
    services::repository::{GitRefInfo, get_git_info},
    views::ThemedRender,
};

#[derive(Template)]
#[template(path = "project/tags.html")]
struct ProjectTagsTemplate<'a> {
    owner: Arc<User>,
    project: &'a Project,
    access_level: AccessType,
    selected_branch: &'a str,
    tags: Vec<GitRefInfo>,
    logged_in_user: Option<Arc<User>>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    active_tab: &'static str,
}

pub async fn project_tags_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, project): PathUserProject,
) -> Response<Body> {
    let repo_context = match ProjectRepoContext::load(&state, &cookies, &owner.slug, &project).await
    {
        Ok(context) => context,
        Err(response) => return response,
    };

    let mut tags: Vec<GitRefInfo> = vec![];
    for b in repo_context.summary.tags() {
        if let Some(info) = get_git_info(&state, &owner.slug, &project.slug, b, 1, 0).await {
            tags.push(info);
        }
    }

    let template = ProjectTagsTemplate {
        owner,
        project: &project,
        access_level: repo_context.project.access_level,
        selected_branch: &project.main_branch,
        tags,
        logged_in_user: repo_context.project.page.logged_in_user,
        sidebar_projects: repo_context.project.page.sidebar_projects,
        content_pages: repo_context.project.page.content_pages,
        active_tab: "",
    };
    template.response()
}

use askama::Template;
use axum::{body::Body, extract::State, response::Response};
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project, User,
    controllers::not_found,
    extractors::PathUserProject,
    models::ContentPage,
    services::{
        repository::{GitRefInfo, get_git_info, get_git_summary},
        session,
    },
    views::ThemedRender,
};

#[derive(Template)]
#[template(path = "project_branches.html")]
struct ProjectBranchesTemplate<'a> {
    owner: &'a User,
    project: &'a Project,
    branches: Vec<GitRefInfo>,
    logged_in_user: Option<&'a User>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
}

pub async fn project_branches_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, project): PathUserProject,
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

    let mut branches: Vec<GitRefInfo> = vec![];
    for b in &summary.branches {
        if let Some(info) = get_git_info(&state, &owner.slug, &project.slug, b, 1, 0).await {
            branches.push(info);
        }
    }

    let template = ProjectBranchesTemplate {
        owner: &owner,
        project: &project,
        branches,
        logged_in_user: logged_in_user.as_ref(),
        sidebar_projects,
        content_pages: state.config.content_pages.clone(),
    };
    template.response()
}

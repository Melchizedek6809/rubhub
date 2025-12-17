use askama::Template;
use axum::{extract::State, http::StatusCode, response::Html};
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project, User,
    controllers::not_found,
    extractors::PathUserProject,
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
    access_level: AccessType,
    branches: Vec<GitRefInfo>,
}

pub async fn project_branches_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, project): PathUserProject,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    let Some(summary) = get_git_summary(&state, &owner.slug, &project.slug).await else {
        return Err(not_found().await);
    };

    let mut branches: Vec<GitRefInfo> = vec![];
    for b in &summary.branches {
        if let Some(info) = get_git_info(&state, &owner.slug, &project.slug, b, 1, 0).await {
            branches.push(info);
        }
    }

    let session_user = session::current_user(&state, &cookies).await.ok();
    let access_level = project
        .access_level(session_user.as_ref().map(|user| user.slug.clone()))
        .await;

    let template = ProjectBranchesTemplate {
        owner: &owner,
        project: &project,
        access_level,
        branches,
    };

    Ok(Html(template.render_with_theme()))
}

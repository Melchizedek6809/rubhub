use askama::Template;
use axum::{body::Body, extract::State, http::Response, response::IntoResponse};
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
#[template(path = "project_tags.html")]
struct ProjectTagsTemplate<'a> {
    owner: &'a User,
    project: &'a Project,
    access_level: AccessType,
    tags: Vec<GitRefInfo>,
    logged_in_user: Option<&'a User>,
}

pub async fn project_tags_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, project): PathUserProject,
) -> Response<Body> {
    let Some(summary) = get_git_summary(&state, &owner.slug, &project.slug).await else {
        return not_found().await.into_response();
    };

    let mut tags: Vec<GitRefInfo> = vec![];
    for b in &summary.tags {
        if let Some(info) = get_git_info(&state, &owner.slug, &project.slug, b, 1, 0).await {
            tags.push(info);
        }
    }

    let session_user = session::current_user(&state, &cookies).await.ok();
    let access_level = project
        .access_level(session_user.as_ref().map(|user| user.slug.clone()))
        .await;

    let template = ProjectTagsTemplate {
        owner: &owner,
        project: &project,
        access_level,
        tags,
        logged_in_user: session_user.as_ref(),
    };
    template.response()
}

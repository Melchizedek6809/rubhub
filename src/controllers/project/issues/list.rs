use askama::Template;
use axum::{
    body::Body,
    extract::{Query, State},
    http::Response,
    response::{Html, IntoResponse},
};
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project,
    extractors::PathUserProject,
    models::{ContentPage, IssueStatus, IssueSummary},
    services::{issue, session},
    views::ThemedRender,
};

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct IssueFilters {
    #[serde(rename = "showCompleted")]
    pub show_completed: bool,
    #[serde(rename = "showCancelled")]
    pub show_cancelled: bool,
}

#[derive(Template)]
#[template(path = "issues_list.html")]
struct IssuesListTemplate<'a> {
    owner: &'a crate::User,
    project: &'a Project,
    access_level: AccessType,
    issues: Vec<IssueSummary>,
    logged_in_user: Option<&'a crate::User>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    active_tab: &'static str,
    selected_branch: String,
    show_completed: bool,
    show_cancelled: bool,
}

pub async fn issues_list_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Query(filters): Query<IssueFilters>,
    PathUserProject(owner, project): PathUserProject,
) -> Response<Body> {
    let logged_in_user = session::current_user(&state, &cookies).await.ok();

    let access_level = project
        .access_level(logged_in_user.as_ref().map(|u| u.slug.clone()))
        .await;

    if access_level == AccessType::None {
        return crate::controllers::not_found(logged_in_user, state.config.content_pages.clone());
    }

    let sidebar_projects = if let Some(ref user) = logged_in_user {
        user.sidebar_projects(&state).await
    } else {
        vec![]
    };

    let all_issues = issue::list_issues(&state, &owner.slug, &project.slug)
        .await
        .unwrap_or_default();

    // Filter issues based on query params (open issues always shown)
    let issues: Vec<IssueSummary> = all_issues
        .into_iter()
        .filter(|issue| match issue.status {
            IssueStatus::Open => true,
            IssueStatus::Completed => filters.show_completed,
            IssueStatus::Cancelled => filters.show_cancelled,
        })
        .collect();

    let template = IssuesListTemplate {
        owner: &owner,
        project: &project,
        access_level,
        issues,
        logged_in_user: logged_in_user.as_ref(),
        sidebar_projects,
        content_pages: state.config.content_pages.clone(),
        active_tab: "issues",
        selected_branch: project.main_branch.clone(),
        show_completed: filters.show_completed,
        show_cancelled: filters.show_cancelled,
    };
    Html(template.render_with_theme()).into_response()
}

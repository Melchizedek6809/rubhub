use std::sync::Arc;

use askama::Template;
use axum::{
    body::Body,
    extract::{Query, State},
    http::Response,
    response::{Html, IntoResponse},
};
use rubhub_auth_store::User;
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project, UserModel,
    controllers::context::ProjectPageContext,
    extractors::PathUserProject,
    models::{ContentPage, IssueStatus, IssueSummary},
    services::issue,
    views::ThemedRender,
};

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct IssueFilters {
    /// Filter to show only issues with this status. Defaults to "open".
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Template)]
#[template(path = "project/talk/list.html")]
struct IssuesListTemplate<'a> {
    owner: Arc<User>,
    project: &'a Project,
    access_level: AccessType,
    issues: Vec<IssueSummary>,
    logged_in_user: Option<Arc<User>>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    active_tab: &'static str,
    selected_branch: String,
    current_status: String,
    count_open: usize,
    count_completed: usize,
    count_cancelled: usize,
}

pub async fn talk_list_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Query(filters): Query<IssueFilters>,
    PathUserProject(owner, project): PathUserProject,
) -> Response<Body> {
    let project_context = match ProjectPageContext::load(&state, &cookies, &project).await {
        Ok(context) => context,
        Err(response) => return response,
    };

    let all_issues = issue::list_issues(&state, &owner.slug, &project.slug)
        .await
        .unwrap_or_default();

    // Count issues by status
    let count_open = all_issues
        .iter()
        .filter(|i| i.status == IssueStatus::Open)
        .count();
    let count_completed = all_issues
        .iter()
        .filter(|i| i.status == IssueStatus::Completed)
        .count();
    let count_cancelled = all_issues
        .iter()
        .filter(|i| i.status == IssueStatus::Cancelled)
        .count();

    // Determine which status to filter by (default: open)
    let current_status = filters.status.unwrap_or_else(|| "open".to_string());
    let filter_status = match current_status.as_str() {
        "completed" => IssueStatus::Completed,
        "cancelled" => IssueStatus::Cancelled,
        _ => IssueStatus::Open,
    };

    // Filter issues to show only the selected status
    let issues: Vec<IssueSummary> = all_issues
        .into_iter()
        .filter(|issue| issue.status == filter_status)
        .collect();

    let template = IssuesListTemplate {
        owner,
        project: &project,
        access_level: project_context.access_level,
        issues,
        logged_in_user: project_context.page.logged_in_user,
        sidebar_projects: project_context.page.sidebar_projects,
        content_pages: project_context.page.content_pages,
        active_tab: "talk",
        selected_branch: project.main_branch.clone(),
        current_status,
        count_open,
        count_completed,
        count_cancelled,
    };
    Html(template.render_with_theme()).into_response()
}

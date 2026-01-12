use std::sync::Arc;

use askama::Template;
use axum::{
    body::Body,
    extract::{Path, State},
    http::Response,
    response::{Html, IntoResponse},
};
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project, User, UserModel,
    models::{ContentPage, Issue, IssueStatus},
    services::{issue, session},
    views::ThemedRender,
};

#[derive(Template)]
#[template(path = "project/talk/view.html")]
struct IssueViewTemplate<'a> {
    owner: Arc<User>,
    project: &'a Project,
    access_level: AccessType,
    issue: Issue,
    logged_in_user: Option<Arc<User>>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    active_tab: &'static str,
    selected_branch: String,
}

pub async fn talk_view_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Path((username, slug, issue_dir)): Path<(String, String, String)>,
) -> Response<Body> {
    let logged_in_user = session::current_user(&state, &cookies).await.ok();
    let content_pages = state.config.content_pages.clone();

    // Load user and project (handle ~ prefix)
    let user_slug = username.strip_prefix("~").unwrap_or(&username);
    let Some(owner) = state.auth.get_user(user_slug) else {
        return crate::controllers::not_found(logged_in_user, content_pages);
    };
    let Ok(project) = Project::load(&state, user_slug, &slug).await else {
        return crate::controllers::not_found(logged_in_user, content_pages);
    };

    let access_level = project
        .access_level(logged_in_user.as_ref().map(|u| u.slug.clone()))
        .await;

    if access_level == AccessType::None {
        return crate::controllers::not_found(logged_in_user, content_pages);
    }

    let Ok(issue) = issue::get_issue(&state, &owner.slug, &project.slug, &issue_dir).await else {
        return crate::controllers::not_found(logged_in_user, content_pages);
    };

    let sidebar_projects = if let Some(ref user) = logged_in_user {
        user.sidebar_projects(&state).await
    } else {
        vec![]
    };

    let template = IssueViewTemplate {
        owner,
        project: &project,
        access_level,
        issue,
        logged_in_user,
        sidebar_projects,
        content_pages: state.config.content_pages.clone(),
        active_tab: "talk",
        selected_branch: project.main_branch.clone(),
    };
    Html(template.render_with_theme()).into_response()
}

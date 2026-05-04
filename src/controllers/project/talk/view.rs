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
    controllers::context::{PageContext, ProjectPageContext},
    models::{ContentPage, Issue, IssueStatus},
    services::issue,
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
    let page_context = PageContext::load(&state, &cookies).await;

    // Load user and project (handle ~ prefix)
    let user_slug = username.strip_prefix("~").unwrap_or(&username);
    let Some(owner) = state.auth.get_user(user_slug) else {
        return page_context.not_found();
    };
    let Ok(project) = Project::load(&state, user_slug, &slug).await else {
        return page_context.not_found();
    };

    let access_level = project
        .access_level(page_context.logged_in_user.as_ref().map(|u| u.slug.clone()))
        .await;
    let project_context = if access_level == AccessType::None {
        return page_context.not_found();
    } else {
        ProjectPageContext {
            page: page_context,
            access_level,
        }
    };

    let Ok(issue) = issue::get_issue(&state, &owner.slug, &project.slug, &issue_dir).await else {
        return project_context.not_found();
    };

    let template = IssueViewTemplate {
        owner,
        project: &project,
        access_level: project_context.access_level,
        issue,
        logged_in_user: project_context.page.logged_in_user,
        sidebar_projects: project_context.page.sidebar_projects,
        content_pages: project_context.page.content_pages,
        active_tab: "talk",
        selected_branch: project.main_branch.clone(),
    };
    Html(template.render_with_theme()).into_response()
}

use askama::Template;
use axum::{
    body::Body,
    extract::{Path, State},
    http::Response,
    response::{Html, IntoResponse, Redirect},
};
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project, User,
    extractors::{CsrfForm, PathUserProject},
    models::{ContentPage, Issue, IssueStatus, IssueSummary},
    services::{csrf, issue, session},
    views::ThemedRender,
};

// ==================== LIST ISSUES ====================

#[derive(Template)]
#[template(path = "issues_list.html")]
struct IssuesListTemplate<'a> {
    owner: &'a User,
    project: &'a Project,
    access_level: AccessType,
    issues: Vec<IssueSummary>,
    logged_in_user: Option<&'a User>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    active_tab: &'static str,
    selected_branch: String,
}

pub async fn issues_list_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
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

    let issues = issue::list_issues(&state, &owner.slug, &project.slug)
        .await
        .unwrap_or_default();

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
    };
    Html(template.render_with_theme()).into_response()
}

// ==================== VIEW SINGLE ISSUE ====================

#[derive(Template)]
#[template(path = "issue_view.html")]
struct IssueViewTemplate<'a> {
    owner: &'a User,
    project: &'a Project,
    access_level: AccessType,
    issue: Issue,
    logged_in_user: Option<&'a User>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    active_tab: &'static str,
    selected_branch: String,
    csrf_token_field: String,
}

pub async fn issue_view_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Path((username, slug, issue_dir)): Path<(String, String, String)>,
) -> Response<Body> {
    let logged_in_user = session::current_user(&state, &cookies).await.ok();
    let content_pages = state.config.content_pages.clone();

    // Load user and project (handle ~ prefix)
    let user_slug = username.strip_prefix("~").unwrap_or(&username);
    let Ok(owner) = User::load(&state, user_slug).await else {
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

    let token = csrf::get_or_create_token(&state.config.csrf_secret, &cookies);
    let csrf_token_field = csrf::hidden_field(&token);

    let template = IssueViewTemplate {
        owner: &owner,
        project: &project,
        access_level,
        issue,
        logged_in_user: logged_in_user.as_ref(),
        sidebar_projects,
        content_pages: state.config.content_pages.clone(),
        active_tab: "issues",
        selected_branch: project.main_branch.clone(),
        csrf_token_field,
    };
    Html(template.render_with_theme()).into_response()
}

// ==================== NEW ISSUE ====================

#[derive(Template)]
#[template(path = "issue_new.html")]
struct NewIssueTemplate<'a> {
    owner: &'a User,
    project: &'a Project,
    access_level: AccessType,
    message: Option<&'a str>,
    logged_in_user: Option<&'a User>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    active_tab: &'static str,
    selected_branch: String,
    csrf_token_field: String,
}

#[derive(Debug, Deserialize)]
pub struct NewIssueForm {
    pub title: String,
    pub content: String,
}

pub async fn issue_new_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, project): PathUserProject,
) -> Response<Body> {
    let current_user = match session::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Redirect::to("/login").into_response(),
    };

    let access_level = project.access_level(Some(current_user.slug.clone())).await;

    // Require at least Read access to create issues
    if !access_level.is_allowed(AccessType::Read) {
        return Redirect::to(&project.uri()).into_response();
    }

    render_new_issue_page(&state, &cookies, &current_user, owner, project, None).await
}

pub async fn issue_new_post(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, project): PathUserProject,
    CsrfForm(form): CsrfForm<NewIssueForm>,
) -> Response<Body> {
    let current_user = match session::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Redirect::to("/login").into_response(),
    };

    let access_level = project.access_level(Some(current_user.slug.clone())).await;
    if !access_level.is_allowed(AccessType::Read) {
        return Redirect::to(&project.uri()).into_response();
    }

    let title = form.title.trim();
    let content = form.content.trim();

    if title.is_empty() {
        return render_new_issue_page(
            &state,
            &cookies,
            &current_user,
            owner,
            project,
            Some("Title is required."),
        )
        .await;
    }

    if content.is_empty() {
        return render_new_issue_page(
            &state,
            &cookies,
            &current_user,
            owner,
            project,
            Some("Description is required."),
        )
        .await;
    }

    match issue::create_issue(&state, &current_user, &project, title, content).await {
        Ok(dir_name) => {
            let uri = format!("/~{}/{}/issues/{}", owner.slug, project.slug, dir_name);
            Redirect::to(&uri).into_response()
        }
        Err(e) => {
            eprintln!("Failed to create issue: {}", e);
            render_new_issue_page(
                &state,
                &cookies,
                &current_user,
                owner,
                project,
                Some("Failed to create issue. Please try again."),
            )
            .await
        }
    }
}

async fn render_new_issue_page(
    state: &GlobalState,
    cookies: &Cookies,
    logged_in_user: &User,
    owner: User,
    project: Project,
    message: Option<&str>,
) -> Response<Body> {
    let sidebar_projects = logged_in_user.sidebar_projects(state).await;
    let token = csrf::get_or_create_token(&state.config.csrf_secret, cookies);
    let csrf_token_field = csrf::hidden_field(&token);

    let template = NewIssueTemplate {
        owner: &owner,
        project: &project,
        access_level: AccessType::Read,
        message,
        logged_in_user: Some(logged_in_user),
        sidebar_projects,
        content_pages: state.config.content_pages.clone(),
        active_tab: "issues",
        selected_branch: project.main_branch.clone(),
        csrf_token_field,
    };
    Html(template.render_with_theme()).into_response()
}

// ==================== ADD COMMENT ====================

#[derive(Debug, Deserialize)]
pub struct AddCommentForm {
    pub content: String,
    pub status: Option<String>,
}

pub async fn issue_comment_post(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Path((username, slug, issue_dir)): Path<(String, String, String)>,
    CsrfForm(form): CsrfForm<AddCommentForm>,
) -> Response<Body> {
    let current_user = match session::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Redirect::to("/login").into_response(),
    };

    let user_slug = username.strip_prefix("~").unwrap_or(&username);
    let Ok(owner) = User::load(&state, user_slug).await else {
        return Redirect::to("/").into_response();
    };
    let Ok(project) = Project::load(&state, user_slug, &slug).await else {
        return Redirect::to("/").into_response();
    };

    let access_level = project.access_level(Some(current_user.slug.clone())).await;
    if !access_level.is_allowed(AccessType::Read) {
        return Redirect::to(&project.uri()).into_response();
    }

    let content = form.content.trim();
    if content.is_empty() {
        let uri = format!("/~{}/{}/issues/{}", owner.slug, project.slug, issue_dir);
        return Redirect::to(&uri).into_response();
    }

    let status = form.status.as_deref().and_then(|s| match s {
        "completed" => Some(IssueStatus::Completed),
        "cancelled" => Some(IssueStatus::Cancelled),
        "open" => Some(IssueStatus::Open),
        _ => None,
    });

    if let Err(e) =
        issue::add_comment(&state, &current_user, &project, &issue_dir, content, status).await
    {
        eprintln!("Failed to add comment: {}", e);
    }

    let uri = format!("/~{}/{}/issues/{}", owner.slug, project.slug, issue_dir);
    Redirect::to(&uri).into_response()
}

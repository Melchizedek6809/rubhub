use std::sync::Arc;

use askama::Template;
use axum::{
    Form,
    body::Body,
    extract::State,
    http::Response,
    response::{Html, IntoResponse, Redirect},
};
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project, User, UserModel, controllers::context::require_user,
    extractors::PathUserProject, models::ContentPage, services::issue, views::ThemedRender,
};

#[derive(Template)]
#[template(path = "project/talk/new.html")]
struct NewIssueTemplate<'a> {
    owner: Arc<User>,
    project: &'a Project,
    access_level: AccessType,
    message: Option<&'a str>,
    logged_in_user: Option<Arc<User>>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    active_tab: &'static str,
    selected_branch: String,
}

#[derive(Debug, Deserialize)]
pub struct NewIssueForm {
    pub title: String,
    pub content: String,
}

pub async fn talk_new_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, project): PathUserProject,
) -> Response<Body> {
    let current_user = match require_user(&state, &cookies).await {
        Ok(user) => user,
        Err(response) => return response,
    };

    let access_level = project.access_level(Some(current_user.slug.clone())).await;

    // Require at least Read access to create issues
    if !access_level.is_allowed(AccessType::Read) {
        return Redirect::to(&project.uri()).into_response();
    }

    render_new_issue_page(&state, current_user, owner, project, None).await
}

pub async fn talk_new_post(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, project): PathUserProject,
    Form(form): Form<NewIssueForm>,
) -> Response<Body> {
    let current_user = match require_user(&state, &cookies).await {
        Ok(user) => user,
        Err(response) => return response,
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
            current_user,
            owner,
            project,
            Some("Title is required."),
        )
        .await;
    }

    if content.is_empty() {
        return render_new_issue_page(
            &state,
            current_user,
            owner,
            project,
            Some("Description is required."),
        )
        .await;
    }

    match issue::create_issue(&state, &current_user, &project, title, content).await {
        Ok(dir_name) => {
            let uri = format!("/~{}/{}/talk/{}", owner.slug, project.slug, dir_name);
            Redirect::to(&uri).into_response()
        }
        Err(e) => {
            eprintln!("Failed to create issue: {}", e);
            render_new_issue_page(
                &state,
                current_user,
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
    current_user: Arc<User>,
    owner: Arc<User>,
    project: Project,
    message: Option<&str>,
) -> Response<Body> {
    let sidebar_projects = current_user.sidebar_projects(state).await;

    let access_level = project.access_level(Some(current_user.slug.clone())).await;
    if !access_level.is_allowed(AccessType::Read) {
        return Redirect::to(&project.uri()).into_response();
    }

    let template = NewIssueTemplate {
        owner,
        project: &project,
        access_level,
        message,
        logged_in_user: Some(current_user),
        sidebar_projects,
        content_pages: state.config.content_pages.clone(),
        active_tab: "talk",
        selected_branch: project.main_branch.clone(),
    };
    Html(template.render_with_theme()).into_response()
}

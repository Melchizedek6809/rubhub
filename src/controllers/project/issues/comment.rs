use axum::{
    body::Body,
    extract::{Path, State},
    http::Response,
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project, User,
    extractors::CsrfForm,
    models::IssueStatus,
    services::{issue, session},
};

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
    let status = form.status.as_deref().and_then(|s| match s {
        "completed" => Some(IssueStatus::Completed),
        "cancelled" => Some(IssueStatus::Cancelled),
        "open" => Some(IssueStatus::Open),
        _ => None,
    });

    // Require either content or a status change
    if content.is_empty() && status.is_none() {
        let uri = format!("/~{}/{}/issues/{}", owner.slug, project.slug, issue_dir);
        return Redirect::to(&uri).into_response();
    }

    if let Err(e) =
        issue::add_comment(&state, &current_user, &project, &issue_dir, content, status).await
    {
        eprintln!("Failed to add comment: {}", e);
    }

    let uri = format!("/~{}/{}/issues/{}", owner.slug, project.slug, issue_dir);
    Redirect::to(&uri).into_response()
}

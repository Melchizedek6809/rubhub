use axum::{
    body::Body,
    extract::State,
    http::Response,
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState,
    extractors::{CsrfForm, PathUserProject},
    services::session,
};

#[derive(Debug, Deserialize)]
pub struct ProjectDeleteForm {
    pub confirmation: String,
}

pub async fn project_delete_post(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, project): PathUserProject,
    CsrfForm(form): CsrfForm<ProjectDeleteForm>,
) -> Response<Body> {
    // Authenticate user
    let current_user = match session::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Redirect::to("/login").into_response(),
    };

    // Verify Admin access (owner only)
    let access_level = project.access_level(Some(current_user.slug.clone())).await;
    if access_level != AccessType::Admin {
        return Redirect::to(&project.uri()).into_response();
    }

    // Validate confirmation: must be exactly "username/projectslug"
    let expected_confirmation = format!("{}/{}", project.owner, project.slug);
    if form.confirmation.trim() != expected_confirmation {
        return Redirect::to(&project.uri_settings()).into_response();
    }

    // Get redirect URL before deletion
    let redirect_url = owner.uri();

    // Delete the project
    match project.delete(&state).await {
        Ok(_) => Redirect::to(&redirect_url).into_response(),
        Err(_) => Redirect::to(&project.uri_settings()).into_response(),
    }
}

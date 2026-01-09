use axum::{
    Form,
    body::Body,
    extract::State,
    http::Response,
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;
use time::OffsetDateTime;
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, UserModel, extractors::PathUserProject, models::RepoEvent,
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
    Form(form): Form<ProjectDeleteForm>,
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

    // Get redirect URL and public access before deletion
    let redirect_url = owner.uri();
    let public_access = project.public_access;

    // Delete the project
    match project.delete(&state).await {
        Ok(_) => {
            // Emit deletion event with captured public_access for access control
            state.emit_event(RepoEvent::RepositoryDeleted {
                owner: project.owner.clone(),
                project: project.slug.clone(),
                public_access,
                timestamp: OffsetDateTime::now_utc(),
            });
            Redirect::to(&redirect_url).into_response()
        }
        Err(_) => Redirect::to(&project.uri_settings()).into_response(),
    }
}

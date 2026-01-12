use std::sync::Arc;

use axum::{
    extract::{FromRef, FromRequestParts, Path},
    http::{StatusCode, request::Parts},
};

use crate::{GlobalState, Project, User};

pub struct PathUserProjectBranch(pub Arc<User>, pub Project, pub String);

impl<S> FromRequestParts<S> for PathUserProjectBranch
where
    S: Send + Sync,
    GlobalState: FromRef<S>,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path((user_slug, project_slug, branch)): Path<(String, String, String)> =
            Path::from_request_parts(parts, state)
                .await
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid path parameter"))?;

        let state = GlobalState::from_ref(state);

        if let Some(user_slug) = user_slug.strip_prefix("~")
            && let Some(user) = state.auth.get_user(user_slug)
            && let Ok(project) = Project::load(&state, user_slug, &project_slug).await
        {
            return Ok(PathUserProjectBranch(user, project, branch));
        }
        Err((StatusCode::NOT_FOUND, "PathUserProject not found"))
    }
}

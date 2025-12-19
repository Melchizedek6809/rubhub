use axum::{
    extract::{FromRef, FromRequestParts, Path},
    http::{StatusCode, request::Parts},
};

use crate::{GlobalState, Project, User};

/// Extractor for /tree/{ref}/*path and /blob/{ref}/*path routes
pub struct PathUserProjectRefPath(pub User, pub Project, pub String, pub String);

impl<S> FromRequestParts<S> for PathUserProjectRefPath
where
    S: Send + Sync,
    GlobalState: FromRef<S>,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path((user_slug, project_slug, git_ref, path)): Path<(String, String, String, String)> =
            Path::from_request_parts(parts, state)
                .await
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid path parameter"))?;

        let state = GlobalState::from_ref(state);

        if let Some(user_slug) = user_slug.strip_prefix("~") {
            if let Ok(user) = User::load(&state, user_slug).await {
                if let Ok(project) = Project::load(&state, user_slug, &project_slug).await {
                    return Ok(PathUserProjectRefPath(user, project, git_ref, path));
                };
            }
        };
        Err((StatusCode::NOT_FOUND, "Path not found"))
    }
}

/// Extractor for /tree/{ref} (root directory)
pub struct PathUserProjectRef(pub User, pub Project, pub String);

impl<S> FromRequestParts<S> for PathUserProjectRef
where
    S: Send + Sync,
    GlobalState: FromRef<S>,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path((user_slug, project_slug, git_ref)): Path<(String, String, String)> =
            Path::from_request_parts(parts, state)
                .await
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid path parameter"))?;

        let state = GlobalState::from_ref(state);

        if let Some(user_slug) = user_slug.strip_prefix("~") {
            if let Ok(user) = User::load(&state, user_slug).await {
                if let Ok(project) = Project::load(&state, user_slug, &project_slug).await {
                    return Ok(PathUserProjectRef(user, project, git_ref));
                };
            }
        };
        Err((StatusCode::NOT_FOUND, "Path not found"))
    }
}

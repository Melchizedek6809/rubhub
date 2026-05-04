use std::sync::Arc;

use axum::{
    extract::{FromRef, FromRequestParts, Path},
    http::{Response, request::Parts},
};

use crate::{GlobalState, Project, User, extractors::themed_not_found};

/// Extractor for /tree/{ref}/*path and /blob/{ref}/*path routes
pub struct PathUserProjectRefPath(pub Arc<User>, pub Project, pub String, pub String);

impl<S> FromRequestParts<S> for PathUserProjectRefPath
where
    S: Send + Sync,
    GlobalState: FromRef<S>,
{
    type Rejection = Response<axum::body::Body>;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Ok(Path((user_slug, project_slug, git_ref, path))) =
            Path::<(String, String, String, String)>::from_request_parts(parts, state).await
        else {
            return Err(themed_not_found(parts, state).await);
        };

        let app_state = GlobalState::from_ref(state);

        if let Some(user_slug) = user_slug.strip_prefix("~")
            && let Some(user) = app_state.auth.get_user(user_slug)
            && let Ok(project) = Project::load(&app_state, user_slug, &project_slug).await
        {
            return Ok(PathUserProjectRefPath(user, project, git_ref, path));
        }
        Err(themed_not_found(parts, state).await)
    }
}

/// Extractor for /tree/{ref} (root directory)
pub struct PathUserProjectRef(pub Arc<User>, pub Project, pub String);

impl<S> FromRequestParts<S> for PathUserProjectRef
where
    S: Send + Sync,
    GlobalState: FromRef<S>,
{
    type Rejection = Response<axum::body::Body>;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Ok(Path((user_slug, project_slug, git_ref))) =
            Path::<(String, String, String)>::from_request_parts(parts, state).await
        else {
            return Err(themed_not_found(parts, state).await);
        };

        let app_state = GlobalState::from_ref(state);

        if let Some(user_slug) = user_slug.strip_prefix("~")
            && let Some(user) = app_state.auth.get_user(user_slug)
            && let Ok(project) = Project::load(&app_state, user_slug, &project_slug).await
        {
            return Ok(PathUserProjectRef(user, project, git_ref));
        }
        Err(themed_not_found(parts, state).await)
    }
}

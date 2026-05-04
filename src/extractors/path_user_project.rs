use std::sync::Arc;

use axum::{
    extract::{FromRef, FromRequestParts, Path},
    http::{Response, request::Parts},
};

use crate::{GlobalState, Project, User, extractors::themed_not_found};

pub struct PathUserProject(pub Arc<User>, pub Project);

impl<S> FromRequestParts<S> for PathUserProject
where
    S: Send + Sync,
    GlobalState: FromRef<S>,
{
    type Rejection = Response<axum::body::Body>;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Ok(Path((user_slug, project_slug))) =
            Path::<(String, String)>::from_request_parts(parts, state).await
        else {
            return Err(themed_not_found(parts, state).await);
        };

        let app_state = GlobalState::from_ref(state);

        if let Some(user_slug) = user_slug.strip_prefix("~")
            && let Some(user) = app_state.auth.get_user(user_slug)
            && let Ok(project) = Project::load(&app_state, user_slug, &project_slug).await
        {
            return Ok(PathUserProject(user, project));
        }
        Err(themed_not_found(parts, state).await)
    }
}

use std::sync::Arc;

use axum::{
    extract::{FromRef, FromRequestParts, Path},
    http::{StatusCode, request::Parts},
};

use crate::{GlobalState, User};

pub struct PathUser(pub Arc<User>);

impl<S> FromRequestParts<S> for PathUser
where
    S: Send + Sync,
    GlobalState: FromRef<S>,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(user_slug): Path<String> = Path::from_request_parts(parts, state)
            .await
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid path parameter"))?;

        let state = GlobalState::from_ref(state);

        if let Some(user_slug) = user_slug.strip_prefix("~")
            && let Some(user) = state.auth.get_user(user_slug)
        {
            return Ok(PathUser(user));
        }
        Err((StatusCode::NOT_FOUND, "PathUser not found"))
    }
}

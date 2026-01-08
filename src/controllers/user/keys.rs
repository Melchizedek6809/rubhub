use axum::{
    body::Body,
    extract::State,
    http::{Response, StatusCode, header},
};

use crate::{GlobalState, extractors::PathUser};

pub async fn user_keys_get(
    State(state): State<GlobalState>,
    PathUser(user): PathUser,
) -> Response<Body> {
    let keys: Vec<String> = state
        .auth
        .get_ssh_keys_for_user(&user.slug)
        .iter()
        .map(|k| k.to_authorized_keys_line())
        .collect();

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(keys.join("\n")))
        .unwrap()
}

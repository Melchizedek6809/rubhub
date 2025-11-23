use axum::{Json, Router, http::StatusCode, routing::get};
use serde::{Deserialize, Serialize};

use crate::state::GlobalState;

#[derive(Deserialize, Serialize)]
struct ErrorResponse {
    error: bool,
    error_message: String,
}

pub fn router() -> Router<GlobalState> {
    Router::new()
        .route("/health", get(|| async { Json("Ok") }))
        .fallback(|| async {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: true,
                    error_message: "Not found".to_owned(),
                }),
            )
        })
}

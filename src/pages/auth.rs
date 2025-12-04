use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, Redirect},
};
use tower_cookies::Cookies;

use crate::{
    services::session,
    state::GlobalState,
};

pub async fn logout(
    State(state): State<GlobalState>,
    cookies: Cookies,
) -> Result<Redirect, (StatusCode, Html<String>)> {
    Ok(session::logout(&state, cookies).await)
}
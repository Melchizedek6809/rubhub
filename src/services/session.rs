use std::sync::Arc;

use anyhow::{Result, anyhow};
use axum::response::Redirect;
use rubhub_auth_store::{Session, User};
use time::{Duration as CookieDuration};
use tower_cookies::{Cookie, Cookies, cookie::SameSite};
use uuid::Uuid;

use crate::GlobalState;

pub const SESSION_COOKIE: &str = "session_id";

pub async fn logout(state: &GlobalState, cookies: Cookies) -> Redirect {
    if let Some(existing) = cookies.get(SESSION_COOKIE) {
        if let Ok(session_id) = Uuid::parse_str(existing.value()) {
            let path = session_id.to_string();
            let path = state.config.session_root.join(&path);
            if let Err(e) = tokio::fs::remove_file(path).await {
                eprintln!("Logout error: {:?}", e);
            };
        }

        cookies.remove(
            Cookie::build((SESSION_COOKIE, ""))
                .path("/")
                .max_age(CookieDuration::seconds(0))
                .build(),
        );
    }

    Redirect::to("/")
}

pub async fn current_user(state: &GlobalState, cookies: &Cookies) -> Result<Arc<User>> {
    let cookie = cookies
        .get(SESSION_COOKIE)
        .ok_or(anyhow!("No Session Cookie"))?;
    let session_id = Uuid::parse_str(cookie.value())?;

    state.auth
        .get_user_by_session(session_id)
        .ok_or(anyhow!("Invalid session"))
}

pub async fn create_session(
    state: &GlobalState,
    cookies: &Cookies,
    user_slug: &str,
) -> Result<()> {
    let session = Session::new(user_slug.to_string());
    let session_id = session.session_id;
    session.save(&state.auth)?;
    
    let cookie = Cookie::build((SESSION_COOKIE, session_id.to_string()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(cfg!(not(debug_assertions)))
        .max_age(CookieDuration::days(30))
        .build();

    cookies.add(cookie);
    Ok(())
}

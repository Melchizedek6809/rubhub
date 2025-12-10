use anyhow::{Result, anyhow};
use axum::response::Redirect;
use serde::{Deserialize, Serialize};
use time::{Duration as CookieDuration, OffsetDateTime};
use tower_cookies::{Cookie, Cookies, cookie::SameSite};
use urlencoding;
use uuid::Uuid;

use crate::{GlobalState, User};

pub const SESSION_COOKIE: &str = "session_id";
pub const SESSION_USER_COOKIE: &str = "session_user";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub session_id: Uuid,
    pub expires_at: OffsetDateTime,
    pub user_id: Uuid,
    pub user_slug: String,
}

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

    if cookies.get(SESSION_USER_COOKIE).is_some() {
        cookies.remove(
            Cookie::build((SESSION_USER_COOKIE, ""))
                .path("/")
                .max_age(CookieDuration::seconds(0))
                .build(),
        );
    }

    Redirect::to("/")
}

pub async fn current_user(state: &GlobalState, cookies: &Cookies) -> Result<User> {
    let cookie = cookies
        .get(SESSION_COOKIE)
        .ok_or(anyhow!("No Session Cookie"))?;
    let session_id = Uuid::parse_str(cookie.value())?;

    let path = session_id.to_string();
    let path = state.config.session_root.join(&path);
    let data = tokio::fs::read(&path).await?;
    let data = String::from_utf8_lossy(&data);
    let session: Session = match serde_json::from_str(&data) {
        Ok(ses) => ses,
        Err(_) => {
            tokio::fs::remove_file(&path).await?;
            return Err(anyhow!("Invalid session"));
        }
    };

    if session.expires_at < time::OffsetDateTime::now_utc() {
        tokio::fs::remove_file(&path).await?;
        return Err(anyhow!("Expired session"));
    }

    let Ok(user) = User::load(state, &session.user_slug).await else {
        return Err(anyhow!("Invalid session"));
    };
    if user.id != session.user_id {
        return Err(anyhow!("Invalid session"));
    }

    Ok(user)
}

pub fn set_user_cookie(cookies: &Cookies, user_id: Uuid, username: &str) {
    let user_info = serde_json::json!({
        "id": user_id,
        "username": username,
    })
    .to_string();

    let encoded_user_info = urlencoding::encode(&user_info).into_owned();

    let user_cookie = Cookie::build((SESSION_USER_COOKIE, encoded_user_info))
        .path("/")
        .http_only(false)
        .same_site(SameSite::Lax)
        .secure(true)
        .max_age(CookieDuration::days(90))
        .build();

    cookies.add(user_cookie);
}

pub async fn create_session(
    state: &GlobalState,
    cookies: &Cookies,
    user_id: Uuid,
    user_slug: &str,
) -> Result<()> {
    let session_id = Uuid::new_v4();
    let expires_at = time::OffsetDateTime::now_utc() + time::Duration::days(30);

    let new_session = Session {
        session_id,
        expires_at,
        user_id,
        user_slug: user_slug.to_string(),
    };
    let json = serde_json::to_string(&new_session)?;
    let path = session_id.to_string();
    let path = state.config.session_root.join(&path);
    tokio::fs::write(path, json).await?;

    let cookie = Cookie::build((SESSION_COOKIE, session_id.to_string()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(true)
        .max_age(CookieDuration::days(30))
        .build();

    cookies.add(cookie);
    set_user_cookie(cookies, user_id, user_slug);
    Ok(())
}

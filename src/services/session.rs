use axum::response::Redirect;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use time::Duration as CookieDuration;
use tower_cookies::{Cookie, Cookies, cookie::SameSite};
use urlencoding;
use uuid::Uuid;

use crate::{
    entities::{session, user},
    state::GlobalState,
};

pub const SESSION_COOKIE: &str = "session_id";
pub const SESSION_USER_COOKIE: &str = "session_user";

pub async fn logout(state: &GlobalState, cookies: Cookies) -> Redirect {
    if let Some(existing) = cookies.get(SESSION_COOKIE) {
        if let Ok(session_id) = Uuid::parse_str(existing.value()) {
            let _ = session::Entity::delete_by_id(session_id)
                .exec(&state.db)
                .await;
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

pub async fn current_user(state: &GlobalState, cookies: &Cookies) -> Result<user::Model, ()> {
    let cookie = cookies.get(SESSION_COOKIE).ok_or(())?;
    let session_id = Uuid::parse_str(cookie.value()).map_err(|_| ())?;

    let session = session::Entity::find_by_id(session_id)
        .one(&state.db)
        .await
        .map_err(|_| ())?
        .ok_or(())?;

    if let Some(expires) = session.expires_at
        && expires < time::OffsetDateTime::now_utc()
    {
        return Err(());
    }

    let user = user::Entity::find_by_id(session.owner)
        .one(&state.db)
        .await
        .map_err(|_| ())?
        .ok_or(())?;

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
    username: &str,
) -> Result<(), sea_orm::DbErr> {
    let session_id = Uuid::new_v4();
    let expires_at = time::OffsetDateTime::now_utc() + time::Duration::days(30);

    let new_session = session::ActiveModel {
        id: Set(session_id),
        expires_at: Set(Some(expires_at)),
        owner: Set(user_id),
    };

    new_session.insert(&state.db).await?;

    let cookie = Cookie::build((SESSION_COOKIE, session_id.to_string()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(true)
        .max_age(CookieDuration::days(90))
        .build();

    cookies.add(cookie);

    set_user_cookie(cookies, user_id, username);

    Ok(())
}

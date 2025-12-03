use tower_cookies::{Cookie, Cookies, cookie::SameSite};
use uuid::Uuid;

pub const CSRF_COOKIE: &str = "csrf_token";

#[derive(Debug)]
pub enum CsrfError {
    Missing,
    Mismatch,
}

impl CsrfError {
    pub fn message(&self) -> &'static str {
        match self {
            CsrfError::Missing => "Missing or expired form token.",
            CsrfError::Mismatch => "Invalid form token.",
        }
    }
}

pub fn ensure_csrf_cookie(cookies: &Cookies) -> String {
    if let Some(existing) = cookies.get(CSRF_COOKIE) {
        return existing.value().to_owned();
    }

    let token = Uuid::new_v4().to_string();
    let cookie = Cookie::build((CSRF_COOKIE, token.clone()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Strict)
        .secure(true)
        .build();

    cookies.add(cookie);

    token
}

pub fn verify_form_token(cookies: &Cookies, provided: Option<&str>) -> Result<(), CsrfError> {
    let expected = cookies
        .get(CSRF_COOKIE)
        .map(|cookie| cookie.value().to_owned())
        .ok_or(CsrfError::Missing)?;

    let provided = provided
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(CsrfError::Missing)?;

    if expected == provided {
        Ok(())
    } else {
        Err(CsrfError::Mismatch)
    }
}

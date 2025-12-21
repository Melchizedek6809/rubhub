use axum::{
    body::Bytes,
    extract::{FromRef, FromRequest, FromRequestParts, Request},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::de::DeserializeOwned;
use tower_cookies::Cookies;

use crate::{GlobalState, services::csrf};

/// A form extractor that validates CSRF tokens before deserializing.
///
/// Use this instead of `Form<T>` for POST handlers that need CSRF protection.
/// The form must include a hidden field named `_csrf_token`.
pub struct CsrfForm<T>(pub T);

pub struct CsrfRejection {
    status: StatusCode,
    message: &'static str,
}

impl IntoResponse for CsrfRejection {
    fn into_response(self) -> Response {
        (self.status, Html(self.message)).into_response()
    }
}

impl<T, S> FromRequest<S> for CsrfForm<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
    GlobalState: FromRef<S>,
{
    type Rejection = CsrfRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let global_state = GlobalState::from_ref(state);

        // Split request into parts and body
        let (mut parts, body) = req.into_parts();

        // Extract cookies
        let cookies = Cookies::from_request_parts(&mut parts, state)
            .await
            .map_err(|_| CsrfRejection {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to extract cookies",
            })?;

        // Get CSRF token from cookie
        let cookie_token = cookies
            .get(csrf::CSRF_COOKIE)
            .map(|c| c.value().to_string())
            .ok_or(CsrfRejection {
                status: StatusCode::FORBIDDEN,
                message: "Invalid request",
            })?;

        // Verify the token signature
        if !csrf::verify_token(&global_state.config.csrf_secret, &cookie_token) {
            return Err(CsrfRejection {
                status: StatusCode::FORBIDDEN,
                message: "Invalid request",
            });
        }

        // Read the body
        let bytes = Bytes::from_request(Request::from_parts(parts, body), state)
            .await
            .map_err(|_| CsrfRejection {
                status: StatusCode::BAD_REQUEST,
                message: "Failed to read request body",
            })?;

        // Parse form data to get the CSRF token from the form
        let form_data: Vec<(String, String)> =
            serde_urlencoded::from_bytes(&bytes).map_err(|_| CsrfRejection {
                status: StatusCode::BAD_REQUEST,
                message: "Invalid form data",
            })?;

        // Find and validate the CSRF token from the form
        let form_token = form_data
            .iter()
            .find(|(k, _)| k == csrf::CSRF_FORM_FIELD)
            .map(|(_, v)| v.as_str())
            .ok_or(CsrfRejection {
                status: StatusCode::FORBIDDEN,
                message: "Invalid request",
            })?;

        // Compare cookie token with form token
        if cookie_token != form_token {
            return Err(CsrfRejection {
                status: StatusCode::FORBIDDEN,
                message: "Invalid request",
            });
        }

        // Deserialize the form data into the target type
        let value: T = serde_urlencoded::from_bytes(&bytes).map_err(|_| CsrfRejection {
            status: StatusCode::BAD_REQUEST,
            message: "Invalid form data",
        })?;

        Ok(CsrfForm(value))
    }
}

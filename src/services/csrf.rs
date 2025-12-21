use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use time::Duration as CookieDuration;
use tower_cookies::{Cookie, Cookies, cookie::SameSite};
use uuid::Uuid;

pub const CSRF_COOKIE: &str = "csrf_token";
pub const CSRF_FORM_FIELD: &str = "_csrf_token";

type HmacSha256 = Hmac<Sha256>;

/// Generate a new CSRF token: base64(random_bytes).base64(hmac_signature)
pub fn generate_token(secret: &[u8; 32]) -> String {
    // Use UUID v4 for random bytes (16 bytes of randomness)
    let random_bytes = Uuid::new_v4().into_bytes();

    let token_data = URL_SAFE_NO_PAD.encode(random_bytes);

    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(token_data.as_bytes());
    let signature = mac.finalize().into_bytes();
    let signature_b64 = URL_SAFE_NO_PAD.encode(signature);

    format!("{}.{}", token_data, signature_b64)
}

/// Verify a CSRF token's HMAC signature
pub fn verify_token(secret: &[u8; 32], token: &str) -> bool {
    let Some((token_data, signature_b64)) = token.split_once('.') else {
        return false;
    };

    let Ok(signature) = URL_SAFE_NO_PAD.decode(signature_b64) else {
        return false;
    };

    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(token_data.as_bytes());

    mac.verify_slice(&signature).is_ok()
}

/// Set the CSRF cookie
pub fn set_csrf_cookie(cookies: &Cookies, token: &str) {
    let cookie = Cookie::build((CSRF_COOKIE, token.to_string()))
        .path("/")
        .same_site(SameSite::Strict)
        .secure(cfg!(not(debug_assertions)))
        .http_only(false) // Must be readable to include in forms
        .max_age(CookieDuration::hours(24))
        .build();

    cookies.add(cookie);
}

/// Get CSRF token from cookies, or generate a new one if not present
pub fn get_or_create_token(secret: &[u8; 32], cookies: &Cookies) -> String {
    if let Some(cookie) = cookies.get(CSRF_COOKIE) {
        let token = cookie.value();
        if verify_token(secret, token) {
            return token.to_string();
        }
    }

    // Generate new token and set cookie
    let token = generate_token(secret);
    set_csrf_cookie(cookies, &token);
    token
}

/// Generate the hidden form field HTML
pub fn hidden_field(token: &str) -> String {
    format!(
        r#"<input type="hidden" name="{}" value="{}">"#,
        CSRF_FORM_FIELD, token
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_verify_token() {
        let secret = [0u8; 32];
        let token = generate_token(&secret);

        assert!(verify_token(&secret, &token));
    }

    #[test]
    fn test_invalid_token_fails() {
        let secret = [0u8; 32];

        assert!(!verify_token(&secret, "invalid"));
        assert!(!verify_token(&secret, "invalid.token"));
        assert!(!verify_token(&secret, ""));
    }

    #[test]
    fn test_tampered_token_fails() {
        let secret = [0u8; 32];
        let token = generate_token(&secret);

        // Tamper with the token data
        let tampered = format!("tampered.{}", token.split_once('.').unwrap().1);
        assert!(!verify_token(&secret, &tampered));
    }

    #[test]
    fn test_wrong_secret_fails() {
        let secret1 = [0u8; 32];
        let secret2 = [1u8; 32];
        let token = generate_token(&secret1);

        assert!(!verify_token(&secret2, &token));
    }
}

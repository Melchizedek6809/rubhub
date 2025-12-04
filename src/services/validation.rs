/// Ensures a slug-like value only includes safe URL characters.
/// Allowed: lowercase ASCII letters, digits, dash, underscore, period (not leading).
pub fn validate_slug(value: &str) -> Result<(), &'static str> {
    if value.len() < 3 {
        return Err("Value must be at least 3 characters.");
    }

    if value.starts_with('.') {
        return Err("Value cannot start with a period.");
    }

    if value
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '-' | '_' | '.'))
    {
        Ok(())
    } else {
        Err("Only lowercase letters, numbers, dashes, underscores, and periods are allowed.")
    }
}

const USERNAME_BLACKLIST: &[&str] = &[
    "projects", "api", "login", "logout", "settings", "public", "dist", "assets", "news", "blog",
    "about", "tos", "privacy", "forum", "chat",
];

pub fn validate_username(username: &str) -> Result<(), &'static str> {
    if username.len() < 3 {
        return Err("Username must be at least 3 characters.");
    }

    let lower = username.to_ascii_lowercase();
    if USERNAME_BLACKLIST.iter().any(|reserved| lower == *reserved) {
        return Err("That username is not allowed.");
    }

    validate_slug(username)
}

pub fn validate_password(password: &str) -> Result<(), &'static str> {
    if password.len() < 16 {
        return Err("Password must be at least 16 characters.");
    }

    Ok(())
}
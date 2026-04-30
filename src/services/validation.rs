use email_address::EmailAddress;

/// Special project slugs that are allowed despite starting with a period
pub const SPECIAL_PROJECT_SLUGS: &[&str] = &[".profile"];

/// Ensures a slug-like value only includes safe URL characters.
/// Allowed: lowercase ASCII letters, digits, dash, underscore, period (not leading).
pub fn validate_slug(value: &str) -> Result<(), &'static str> {
    if value.len() < 3 {
        return Err("Value must be at least 3 characters.");
    }

    // Allow special slugs like .profile
    if SPECIAL_PROJECT_SLUGS.contains(&value) {
        return Ok(());
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

    if username.starts_with('.') {
        return Err("Usernames cannot start with a period.");
    }

    if username.chars().all(|ch| {
        ch.is_ascii_uppercase()
            || ch.is_ascii_lowercase()
            || ch.is_ascii_digit()
            || matches!(ch, '-' | '_' | '.')
    }) {
        Ok(())
    } else {
        Err("Only letters, numbers, dashes, underscores, and periods are allowed.")
    }
}

pub fn validate_email(email: &str) -> Result<(), &'static str> {
    if email.len() < 3 {
        return Err("E-Mail must be at least 3 characters.");
    }

    if EmailAddress::is_valid(email) {
        Ok(())
    } else {
        Err("Invalid E-Mail address.")
    }
}

pub fn validate_password(password: &str) -> Result<(), &'static str> {
    if password.len() < 16 {
        return Err("Password must be at least 16 characters.");
    }

    Ok(())
}

pub fn slugify(name: &str) -> String {
    let mut result = String::new();
    let mut last_dash = false;

    for ch in name.chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            result.push(lower);
            last_dash = false;
        } else if !last_dash {
            result.push('-');
            last_dash = true;
        }
    }

    while result.starts_with('-') {
        result.remove(0);
    }
    while result.ends_with('-') {
        result.pop();
    }

    if result.is_empty() {
        "project".to_owned()
    } else {
        result
    }
}

pub fn validate_project_name(name: &str) -> Result<(), &'static str> {
    if name.len() < 3 {
        return Err("Project name must be at least 3 characters.");
    }

    Ok(())
}

/// Check if a project name is reserved (used only during project creation)
pub fn is_reserved_project_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    SPECIAL_PROJECT_SLUGS.contains(&lower.as_str())
}

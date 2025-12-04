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

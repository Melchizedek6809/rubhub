use serde::{Deserialize, Serialize};

/// Format a time difference in seconds as a human-readable relative time string
pub fn format_relative_time(seconds: i64) -> String {
    if seconds < 60 {
        return format!(
            "{} second{} ago",
            seconds,
            if seconds != 1 { "s" } else { "" }
        );
    }
    if seconds < 3600 {
        let minutes = seconds / 60;
        return format!(
            "{} minute{} ago",
            minutes,
            if minutes != 1 { "s" } else { "" }
        );
    }
    if seconds < 86400 {
        let hours = seconds / 3600;
        return format!("{} hour{} ago", hours, if hours != 1 { "s" } else { "" });
    }
    if seconds < 86400 * 30 {
        let days = seconds / 86400;
        return format!("{} day{} ago", days, if days != 1 { "s" } else { "" });
    }
    if seconds < 86400 * 365 {
        let months = seconds / (86400 * 30);
        return format!("{} month{} ago", months, if months != 1 { "s" } else { "" });
    }
    let years = seconds / (86400 * 365);
    format!("{} year{} ago", years, if years != 1 { "s" } else { "" })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessType {
    None,
    Read,
    Write,
    Admin,
}

impl std::fmt::Display for AccessType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            AccessType::None => "None",
            AccessType::Read => "Read",
            AccessType::Write => "Write",
            AccessType::Admin => "Admin",
        };
        write!(f, "{label}")
    }
}

impl AccessType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccessType::None => "none",
            AccessType::Read => "read",
            AccessType::Write => "write",
            AccessType::Admin => "admin",
        }
    }

    pub fn parse_public_access(value: &str) -> Result<Self, &'static str> {
        match value.to_ascii_lowercase().as_str() {
            "none" => Ok(AccessType::None),
            "read" => Ok(AccessType::Read),
            "write" => Ok(AccessType::Write),
            "admin" => Err("Public admin access is not allowed."),
            _ => Err("Invalid access level."),
        }
    }

    pub fn is_allowed(&self, required_level: AccessType) -> bool {
        (*self as u8) >= (required_level as u8)
    }
}

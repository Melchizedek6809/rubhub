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

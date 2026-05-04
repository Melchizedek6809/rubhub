pub struct PageMeta {
    pub title: String,
    pub description: String,
    pub canonical_url: Option<String>,
    pub robots: Option<&'static str>,
}

impl PageMeta {
    pub fn new(title: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: meta_description(description.into()),
            canonical_url: None,
            robots: None,
        }
    }

    pub fn canonical(mut self, base_url: &str, path: &str) -> Self {
        self.canonical_url = Some(canonical_url(base_url, path));
        self
    }

    pub fn robots(mut self, robots: &'static str) -> Self {
        self.robots = Some(robots);
        self
    }
}

pub fn canonical_url(base_url: &str, path: &str) -> String {
    let base = base_url.trim_end_matches('/');
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    format!("{base}{path}")
}

pub fn meta_description(description: impl AsRef<str>) -> String {
    let normalized = description
        .as_ref()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    if normalized.chars().count() <= 160 {
        return normalized;
    }

    let mut truncated = normalized.chars().take(157).collect::<String>();
    truncated.push_str("...");
    truncated
}

pub fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_url_joins_base_and_path() {
        assert_eq!(
            canonical_url("https://rubhub.net/", "/~ben/rubhub"),
            "https://rubhub.net/~ben/rubhub"
        );
        assert_eq!(
            canonical_url("https://rubhub.net", "projects"),
            "https://rubhub.net/projects"
        );
    }

    #[test]
    fn meta_description_normalizes_and_truncates() {
        let description = meta_description("hello\n\n   world");
        assert_eq!(description, "hello world");

        let long = "a".repeat(200);
        let description = meta_description(long);
        assert_eq!(description.chars().count(), 160);
        assert!(description.ends_with("..."));
    }

    #[test]
    fn xml_escape_escapes_special_characters() {
        assert_eq!(
            xml_escape("https://example.test/?a=1&b=\"x\""),
            "https://example.test/?a=1&amp;b=&quot;x&quot;"
        );
    }
}

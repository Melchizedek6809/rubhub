/// Parsed frontmatter as key-value pairs
pub type Frontmatter = Vec<(String, String)>;

/// Parse YAML frontmatter from markdown content.
/// Returns (frontmatter properties, body content).
/// If no valid frontmatter is found, returns empty vec and full content as body.
pub fn parse_frontmatter(content: &str) -> (Frontmatter, String) {
    // Must start with ---\n
    if !content.starts_with("---\n") {
        return (vec![], content.to_string());
    }

    let rest = &content[4..];

    // Find closing ---
    let end = match rest.find("\n---\n").or_else(|| rest.find("\n---")) {
        Some(pos) => pos,
        None => return (vec![], content.to_string()),
    };

    let yaml = &rest[..end];

    // Find where body starts (after the closing ---)
    let body_start = if rest[end..].starts_with("\n---\n") {
        end + 5
    } else {
        end + 4
    };
    let body = if body_start < rest.len() {
        rest[body_start..].trim().to_string()
    } else {
        String::new()
    };

    // Parse YAML as a mapping of string keys to values
    let frontmatter = match serde_yaml::from_str::<serde_yaml::Value>(yaml) {
        Ok(serde_yaml::Value::Mapping(map)) => map
            .into_iter()
            .filter_map(|(k, v)| {
                let key = k.as_str()?.to_string();
                let value = yaml_value_to_string(&v)?;
                Some((key, value))
            })
            .collect(),
        _ => vec![],
    };

    (frontmatter, body)
}

/// Convert a YAML value to a display string
fn yaml_value_to_string(value: &serde_yaml::Value) -> Option<String> {
    match value {
        serde_yaml::Value::String(s) => Some(s.clone()),
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        serde_yaml::Value::Bool(b) => Some(b.to_string()),
        serde_yaml::Value::Null => None,
        // For arrays and maps, just show a simplified representation
        serde_yaml::Value::Sequence(seq) => {
            let items: Vec<String> = seq.iter().filter_map(yaml_value_to_string).collect();
            if items.is_empty() {
                None
            } else {
                Some(items.join(", "))
            }
        }
        serde_yaml::Value::Mapping(_) => None, // Skip nested objects
        serde_yaml::Value::Tagged(tagged) => yaml_value_to_string(&tagged.value),
    }
}

/// Render markdown content to sanitized HTML
pub fn render_markdown(content: &str) -> String {
    let html =
        markdown::to_html_with_options(content, &markdown::Options::gfm()).unwrap_or_default();
    ammonia::clean(&html)
}

/// Parse frontmatter and render markdown body to HTML
/// Returns (frontmatter properties, rendered HTML)
pub fn parse_and_render(content: &str) -> (Frontmatter, String) {
    let (frontmatter, body) = parse_frontmatter(content);
    let html = render_markdown(&body);
    (frontmatter, html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter_basic() {
        let content = "---\ntitle: Hello\nauthor: World\n---\n\nBody content here.";
        let (fm, body) = parse_frontmatter(content);
        assert_eq!(fm.len(), 2);
        assert_eq!(fm[0], ("title".to_string(), "Hello".to_string()));
        assert_eq!(fm[1], ("author".to_string(), "World".to_string()));
        assert_eq!(body, "Body content here.");
    }

    #[test]
    fn test_parse_frontmatter_no_frontmatter() {
        let content = "Just regular markdown content.";
        let (fm, body) = parse_frontmatter(content);
        assert!(fm.is_empty());
        assert_eq!(body, content);
    }

    #[test]
    fn test_parse_frontmatter_empty_body() {
        let content = "---\ntitle: Test\n---";
        let (fm, body) = parse_frontmatter(content);
        assert_eq!(fm.len(), 1);
        assert!(body.is_empty());
    }
}

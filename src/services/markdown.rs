use std::{borrow::Cow, collections::HashSet};

/// Parsed frontmatter as key-value pairs
pub type Frontmatter = Vec<(String, String)>;

#[derive(Clone, Debug)]
pub struct MarkdownRenderContext {
    base_url: String,
}

impl MarkdownRenderContext {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }

    pub fn render(&self, content: &str) -> String {
        render_markdown_with_context(content, Some(self))
    }

    pub fn parse_and_render(&self, content: &str) -> (Frontmatter, String) {
        parse_and_render_with_context(content, Some(self))
    }
}

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

fn render_markdown_with_context(content: &str, context: Option<&MarkdownRenderContext>) -> String {
    let html =
        markdown::to_html_with_options(content, &markdown::Options::gfm()).unwrap_or_default();
    sanitize_html(&html, context)
}

fn parse_and_render_with_context(
    content: &str,
    context: Option<&MarkdownRenderContext>,
) -> (Frontmatter, String) {
    let (frontmatter, body) = parse_frontmatter(content);
    let html = render_markdown_with_context(&body, context);
    (frontmatter, html)
}

fn sanitize_html(html: &str, context: Option<&MarkdownRenderContext>) -> String {
    let url_schemes: HashSet<&str> = ["http", "https", "mailto", "tel"].into_iter().collect();
    let mut builder = ammonia::Builder::new();
    builder.url_schemes(url_schemes);

    if let Some(context) = context {
        builder.url_relative(ammonia::UrlRelative::Custom(Box::new(
            RelativeUrlResolver {
                base_url: context.base_url.clone(),
            },
        )));
    }

    builder.clean(html).to_string()
}

struct RelativeUrlResolver {
    base_url: String,
}

impl<'a> ammonia::UrlRelativeEvaluate<'a> for RelativeUrlResolver {
    fn evaluate<'url>(&self, url: &'url str) -> Option<Cow<'url, str>> {
        resolve_relative_url(url, &self.base_url).map(Cow::Owned)
    }
}

fn resolve_relative_url(url: &str, base_url: &str) -> Option<String> {
    if url.is_empty() || url.starts_with('#') || url.starts_with('/') {
        return if url.starts_with("//") {
            None
        } else {
            Some(url.to_string())
        };
    }

    let (path_and_query, fragment) = split_once_keep_delimiter(url, '#');
    let (path, query) = split_once_keep_delimiter(path_and_query, '?');
    let mut rewritten = join_url_path(base_url, path);
    rewritten.push_str(query);
    rewritten.push_str(fragment);
    Some(rewritten)
}

fn split_once_keep_delimiter(value: &str, delimiter: char) -> (&str, &str) {
    match value.find(delimiter) {
        Some(idx) => (&value[..idx], &value[idx..]),
        None => (value, ""),
    }
}

fn join_url_path(base_url: &str, relative_path: &str) -> String {
    let mut parts: Vec<&str> = base_url
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();

    for part in relative_path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }

    format!("/{}", parts.join("/"))
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

    #[test]
    fn rewrites_relative_links_against_base_url() {
        let context = MarkdownRenderContext::new("/~ben/hoshi/blob/main/docs/");
        let html = context.render("[Guide](guide.md)");

        assert!(html.contains("href=\"/~ben/hoshi/blob/main/docs/guide.md\""));
    }

    #[test]
    fn rewrites_relative_images_against_base_url() {
        let context = MarkdownRenderContext::new("/~ben/hoshi/blob/main/docs/");
        let html = context.render("![Logo](assets/logo.png)");

        assert!(html.contains("src=\"/~ben/hoshi/blob/main/docs/assets/logo.png\""));
    }

    #[test]
    fn preserves_query_and_fragment_when_rewriting() {
        let context = MarkdownRenderContext::new("/~ben/hoshi/blob/main/");
        let html = context.render("![Diagram](diagram.svg?theme=dark#box)");

        assert!(html.contains("src=\"/~ben/hoshi/blob/main/diagram.svg?theme=dark#box\""));
    }

    #[test]
    fn resolves_parent_segments_without_leading_root_escape() {
        let context = MarkdownRenderContext::new("/~ben/hoshi/blob/main/docs/guides/");
        let html = context.render("[Readme](../README.md)");

        assert!(html.contains("href=\"/~ben/hoshi/blob/main/docs/README.md\""));
    }

    #[test]
    fn leaves_absolute_and_root_absolute_urls_alone() {
        let context = MarkdownRenderContext::new("/~ben/hoshi/blob/main/");
        let html = context.render(
            "[Site](https://example.com) [Mail](mailto:test@example.com) [Root](/projects)",
        );

        assert!(html.contains("href=\"https://example.com\""));
        assert!(html.contains("href=\"mailto:test@example.com\""));
        assert!(html.contains("href=\"/projects\""));
    }

    #[test]
    fn strips_unsafe_and_protocol_relative_urls() {
        let context = MarkdownRenderContext::new("/~ben/hoshi/blob/main/");
        let html = context.render("[Bad](javascript:alert(1)) ![Bad](//example.com/a.png)");

        assert!(!html.contains("javascript:"));
        assert!(!html.contains("src="));
    }
}

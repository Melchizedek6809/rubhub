use anyhow::{Context, Result};

/// A content page specification pointing to a markdown file in a repository
#[derive(Debug, Clone, PartialEq)]
pub struct ContentPage {
    pub title: String,
    pub slug: String,
    pub repo_owner: String,
    pub repo_slug: String,
    pub file_path: String,
}

impl ContentPage {
    /// Parse a content page specification in the format "Title:user/repo/path.md"
    pub fn parse(spec: &str) -> Result<Self> {
        let (title, path) = spec
            .split_once(':')
            .context("Invalid format: expected 'Title:user/repo/path.md'")?;

        let title = title.trim();
        if title.is_empty() {
            anyhow::bail!("Title cannot be empty");
        }

        let path = path.trim();
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() < 3 {
            anyhow::bail!(
                "Invalid path format: expected 'user/repo/file.md', got '{}'",
                path
            );
        }

        let repo_owner = parts[0];
        let repo_slug = parts[1];
        let file_path = parts[2..].join("/");

        if file_path.is_empty() {
            anyhow::bail!("File path cannot be empty");
        }

        let slug = slugify(title);

        Ok(Self {
            title: title.to_string(),
            slug,
            repo_owner: repo_owner.to_string(),
            repo_slug: repo_slug.to_string(),
            file_path,
        })
    }

    /// Parse an index content specification in the format "user/repo/path.md" or "~user/repo/path.md"
    pub fn parse_index(spec: &str) -> Result<Self> {
        let path = spec.trim();

        // Strip leading ~ if present
        let path = path.strip_prefix('~').unwrap_or(path);

        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() < 3 {
            anyhow::bail!(
                "Invalid path format: expected 'user/repo/file.md' or '~user/repo/file.md', got '{}'",
                spec
            );
        }

        let repo_owner = parts[0];
        let repo_slug = parts[1];
        let file_path = parts[2..].join("/");

        if file_path.is_empty() {
            anyhow::bail!("File path cannot be empty");
        }

        Ok(Self {
            title: "Index".to_string(),
            slug: "index".to_string(),
            repo_owner: repo_owner.to_string(),
            repo_slug: repo_slug.to_string(),
            file_path,
        })
    }

    /// Get the URL path for this content page (e.g., "/contact")
    pub fn url_path(&self) -> String {
        format!("/{}", self.slug)
    }
}

/// Convert a title to a URL-safe slug
fn slugify(name: &str) -> String {
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

    // Trim trailing dashes
    result.trim_end_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid() {
        let page = ContentPage::parse("Contact:ben/repo/contact.md").unwrap();
        assert_eq!(page.title, "Contact");
        assert_eq!(page.slug, "contact");
        assert_eq!(page.repo_owner, "ben");
        assert_eq!(page.repo_slug, "repo");
        assert_eq!(page.file_path, "contact.md");
    }

    #[test]
    fn test_parse_with_spaces() {
        let page = ContentPage::parse("Terms of Service:ben/repo/terms.md").unwrap();
        assert_eq!(page.title, "Terms of Service");
        assert_eq!(page.slug, "terms-of-service");
    }

    #[test]
    fn test_parse_nested_path() {
        let page = ContentPage::parse("About:user/repo/docs/about.md").unwrap();
        assert_eq!(page.file_path, "docs/about.md");
    }

    #[test]
    fn test_parse_missing_colon() {
        assert!(ContentPage::parse("Contact").is_err());
    }

    #[test]
    fn test_parse_invalid_path() {
        assert!(ContentPage::parse("Contact:invalid").is_err());
    }

    #[test]
    fn test_url_path() {
        let page = ContentPage::parse("Contact:ben/repo/contact.md").unwrap();
        assert_eq!(page.url_path(), "/contact");
    }

    #[test]
    fn test_parse_index() {
        let page = ContentPage::parse_index("ben/repo/index.md").unwrap();
        assert_eq!(page.title, "Index");
        assert_eq!(page.slug, "index");
        assert_eq!(page.repo_owner, "ben");
    }

    #[test]
    fn test_parse_index_with_tilde() {
        let page = ContentPage::parse_index("~ben/repo/index.md").unwrap();
        assert_eq!(page.repo_owner, "ben");
    }
}

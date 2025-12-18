use anyhow::{Context, Result};

use crate::{GlobalState, services::{repository, validation}};

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

        // Validate owner and repo slugs
        validation::validate_slug(repo_owner)
            .map_err(|e| anyhow::anyhow!("Invalid repository owner '{}': {}", repo_owner, e))?;
        validation::validate_slug(repo_slug)
            .map_err(|e| anyhow::anyhow!("Invalid repository slug '{}': {}", repo_slug, e))?;

        if file_path.is_empty() {
            anyhow::bail!("File path cannot be empty");
        }

        let slug = validation::slugify(title);

        Ok(Self {
            title: title.to_string(),
            slug,
            repo_owner: repo_owner.to_string(),
            repo_slug: repo_slug.to_string(),
            file_path,
        })
    }

    /// Get the URL path for this content page (e.g., "/contact")
    pub fn url_path(&self) -> String {
        format!("/{}", self.slug)
    }

    /// Fetch and render the markdown content for this page
    pub async fn render_content(&self, state: &GlobalState) -> Result<String> {
        // Try "main" branch first, then "master" as fallback
        let branches = ["main", "master"];
        let mut last_error = None;

        for branch in &branches {
            match repository::get_git_file(
                state,
                &self.repo_owner,
                &self.repo_slug,
                branch,
                &self.file_path,
            )
            .await
            {
                Ok(blob) => {
                    // Convert blob to UTF-8
                    let markdown_str = String::from_utf8_lossy(&blob.data);

                    // Render markdown with GitHub Flavored Markdown
                    let html = markdown::to_html_with_options(
                        &markdown_str,
                        &markdown::Options::gfm(),
                    )
                    .unwrap_or_default();

                    // Sanitize HTML to prevent XSS
                    return Ok(ammonia::clean(&html));
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        // If we get here, both branches failed
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Failed to fetch content")))
    }
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
}

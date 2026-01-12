use anyhow::Result;

use crate::{GlobalState, models::ContentPage, services::repository};

/// Fetch and render the markdown content for a content page
pub async fn render_content(page: &ContentPage, state: &GlobalState) -> Result<String> {
    // Try "main" branch first, then "master" as fallback
    let branches = ["main", "master"];
    let mut last_error = None;

    for branch in &branches {
        match repository::get_git_file(
            state,
            &page.repo_owner,
            &page.repo_slug,
            branch,
            &page.file_path,
        )
        .await
        {
            Ok(blob) => {
                // Convert blob to UTF-8
                let markdown_str = String::from_utf8_lossy(&blob);

                // Render markdown with GitHub Flavored Markdown
                let html =
                    markdown::to_html_with_options(&markdown_str, &markdown::Options::gfm())
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

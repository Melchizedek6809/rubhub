use anyhow::Result;

use crate::{
    GlobalState,
    models::ContentPage,
    services::{markdown, repository},
};

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

                return Ok(markdown::MarkdownRenderContext::new(content_page_base_url(
                    page, branch,
                ))
                .render(&markdown_str));
            }
            Err(e) => {
                last_error = Some(e);
            }
        }
    }

    // If we get here, both branches failed
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Failed to fetch content")))
}

fn content_page_base_url(page: &ContentPage, branch: &str) -> String {
    let encoded_branch = urlencoding::encode(branch);
    let base = format!(
        "/~{}/{}/blob/{}",
        page.repo_owner, page.repo_slug, encoded_branch
    );

    match page.file_path.rsplit_once('/') {
        Some((parent, _)) => format!("{base}/{parent}/"),
        None => format!("{base}/"),
    }
}

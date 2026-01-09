use anyhow::{Result, anyhow};
use serde::Deserialize;
use time::OffsetDateTime;

use crate::{
    GlobalState, Project,
    services::repository::{
        add_file_to_branch, branch_exists, create_orphan_branch, get_git_file, get_git_summary,
    },
};

const INFO_BRANCH: &str = "meta/info";
const README_FILE: &str = "README.md";

/// Frontmatter parsed from meta/info branch README.md
#[derive(Debug, Default, Deserialize)]
pub struct ProjectInfoFrontmatter {
    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub public_access: Option<String>,

    #[serde(default)]
    pub default_branch: Option<String>,

    #[serde(default)]
    pub website: Option<String>,

    #[serde(default, with = "time::serde::rfc3339::option")]
    pub created_at: Option<OffsetDateTime>,
}

/// Parse YAML frontmatter from markdown content
fn parse_frontmatter(content: &str) -> Result<(ProjectInfoFrontmatter, String)> {
    if !content.starts_with("---\n") {
        return Err(anyhow!("Missing frontmatter"));
    }

    let rest = &content[4..];
    let end = rest
        .find("\n---\n")
        .or_else(|| rest.find("\n---"))
        .ok_or_else(|| anyhow!("Invalid frontmatter"))?;
    let yaml = &rest[..end];

    // Find where body starts (after the closing ---)
    let body_start = if rest[end..].starts_with("\n---\n") {
        end + 5
    } else {
        end + 4
    };
    let body = if body_start < rest.len() {
        rest[body_start..].trim()
    } else {
        ""
    };

    let frontmatter: ProjectInfoFrontmatter = serde_yaml::from_str(yaml)?;
    Ok((frontmatter, body.to_string()))
}

/// Generate README.md content from Project fields
fn generate_readme(project: &Project) -> String {
    let formatted_date = project
        .created_at
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "".to_string());

    let mut content = String::from("---\n");
    content.push_str(&format!("name: {}\n", project.name));
    content.push_str(&format!(
        "public_access: {}\n",
        project.public_access.as_str()
    ));
    content.push_str(&format!("default_branch: {}\n", project.main_branch));

    if !project.website.is_empty() {
        content.push_str(&format!("website: {}\n", project.website));
    }

    content.push_str(&format!("created_at: {}\n", formatted_date));
    content.push_str("---\n\n");

    if !project.description.is_empty() {
        content.push_str(&project.description);
        content.push('\n');
    }

    content
}

/// Load project metadata from meta/info branch
/// Returns (frontmatter, description) or defaults if branch/file doesn't exist
pub async fn load_project_info(
    state: &GlobalState,
    user_slug: &str,
    project_slug: &str,
) -> Result<(ProjectInfoFrontmatter, String)> {
    // Check if branch exists
    if !branch_exists(state, user_slug, project_slug, INFO_BRANCH).await {
        return Ok((ProjectInfoFrontmatter::default(), String::new()));
    }

    // Try to read README.md
    let content = match get_git_file(state, user_slug, project_slug, INFO_BRANCH, README_FILE).await
    {
        Ok(c) => c,
        Err(_) => return Ok((ProjectInfoFrontmatter::default(), String::new())),
    };

    let content_str = String::from_utf8_lossy(&content.data);

    // Parse frontmatter
    match parse_frontmatter(&content_str) {
        Ok((fm, desc)) => Ok((fm, desc)),
        Err(_) => Ok((ProjectInfoFrontmatter::default(), String::new())),
    }
}

/// Save project metadata to meta/info branch
pub async fn save_project_info(
    state: &GlobalState,
    user_slug: &str,
    project_slug: &str,
    project: &Project,
    author_name: &str,
    author_email: &str,
) -> Result<()> {
    let readme_content = generate_readme(project);

    // Check if branch exists
    if !branch_exists(state, user_slug, project_slug, INFO_BRANCH).await {
        // Create orphan branch with initial content
        create_orphan_branch(
            state,
            user_slug,
            project_slug,
            INFO_BRANCH,
            README_FILE,
            &readme_content,
            "Initialize project info",
            author_name,
            author_email,
        )
        .await?;
    } else {
        // Update existing branch
        add_file_to_branch(
            state,
            user_slug,
            project_slug,
            INFO_BRANCH,
            README_FILE,
            &readme_content,
            "Update project info",
            author_name,
            author_email,
        )
        .await?;
    }

    Ok(())
}

/// Detect default branch by checking main, master, or first available
pub async fn detect_default_branch(
    state: &GlobalState,
    user_slug: &str,
    project_slug: &str,
) -> String {
    let summary = match get_git_summary(state, user_slug, project_slug).await {
        Some(s) => s,
        None => return "main".to_string(),
    };

    // Priority: main > master > first non-meta branch
    let branches = summary.branches();
    if branches.contains(&"main") {
        return "main".to_string();
    }
    if branches.contains(&"master") {
        return "master".to_string();
    }

    // Return first branch that isn't a meta/* branch
    for branch in branches {
        if !branch.starts_with("meta/") {
            return branch.to_string();
        }
    }

    // Fallback
    "main".to_string()
}

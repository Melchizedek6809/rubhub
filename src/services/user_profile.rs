use anyhow::{Result, anyhow};
use russh::keys::ssh_key;
use serde::Deserialize;
use time::OffsetDateTime;

use crate::{
    GlobalState,
    services::repository::{add_file_to_branch, branch_exists, create_orphan_branch, get_git_file},
};

pub const PROFILE_REPO: &str = ".profile";
const MAIN_BRANCH: &str = "main";
const README_FILE: &str = "README.md";
const SSH_KEYS_FILE: &str = "authorized_keys";

/// Frontmatter parsed from .profile/README.md
#[derive(Debug, Default, Deserialize)]
pub struct UserProfileFrontmatter {
    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub email: Option<String>,

    #[serde(default)]
    pub website: Option<String>,

    #[serde(default)]
    pub default_main_branch: Option<String>,

    #[serde(default, with = "time::serde::rfc3339::option")]
    pub created_at: Option<OffsetDateTime>,
}

/// Parse YAML frontmatter from markdown content
fn parse_frontmatter(content: &str) -> Result<(UserProfileFrontmatter, String)> {
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

    let frontmatter: UserProfileFrontmatter = serde_yaml::from_str(yaml)?;
    Ok((frontmatter, body.to_string()))
}

/// Generate README.md content from user profile fields
fn generate_readme(
    name: &str,
    email: &str,
    website: &str,
    default_main_branch: &str,
    description: &str,
    created_at: OffsetDateTime,
) -> String {
    let formatted_date = created_at
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "".to_string());

    let mut content = String::from("---\n");
    content.push_str(&format!("name: {}\n", name));

    if !email.is_empty() {
        content.push_str(&format!("email: {}\n", email));
    }

    if !website.is_empty() {
        content.push_str(&format!("website: {}\n", website));
    }

    content.push_str(&format!("default_main_branch: {}\n", default_main_branch));
    content.push_str(&format!("created_at: {}\n", formatted_date));
    content.push_str("---\n\n");

    if !description.is_empty() {
        content.push_str(description);
        content.push('\n');
    }

    content
}

/// Check if the .profile repo exists for a user
pub async fn profile_repo_exists(state: &GlobalState, user_slug: &str) -> bool {
    let path = state.config.git_root.join(user_slug).join(PROFILE_REPO);
    tokio::fs::metadata(&path).await.is_ok()
}

/// Load user profile from .profile/README.md
/// Returns (frontmatter, description) or defaults if repo/file doesn't exist
pub async fn load_user_profile(
    state: &GlobalState,
    user_slug: &str,
) -> (UserProfileFrontmatter, String) {
    // Check if repo exists
    if !profile_repo_exists(state, user_slug).await {
        return (UserProfileFrontmatter::default(), String::new());
    }

    // Check if main branch exists
    if !branch_exists(state, user_slug, PROFILE_REPO, MAIN_BRANCH).await {
        return (UserProfileFrontmatter::default(), String::new());
    }

    // Try to read README.md
    let content = match get_git_file(state, user_slug, PROFILE_REPO, MAIN_BRANCH, README_FILE).await
    {
        Ok(c) => c,
        Err(_) => return (UserProfileFrontmatter::default(), String::new()),
    };

    let content_str = String::from_utf8_lossy(&content.data);

    // Parse frontmatter
    match parse_frontmatter(&content_str) {
        Ok((fm, desc)) => (fm, desc),
        Err(_) => (UserProfileFrontmatter::default(), String::new()),
    }
}

/// Save user profile to .profile/README.md
pub async fn save_user_profile(
    state: &GlobalState,
    user_slug: &str,
    name: &str,
    email: &str,
    website: &str,
    default_main_branch: &str,
    description: &str,
    created_at: OffsetDateTime,
) -> Result<()> {
    let readme_content = generate_readme(
        name,
        email,
        website,
        default_main_branch,
        description,
        created_at,
    );

    // Ensure .profile repo exists with main branch
    if !profile_repo_exists(state, user_slug).await {
        return Err(anyhow!(".profile repo does not exist"));
    }

    // Check if main branch exists
    if !branch_exists(state, user_slug, PROFILE_REPO, MAIN_BRANCH).await {
        // Create orphan branch with initial content
        create_orphan_branch(
            state,
            user_slug,
            PROFILE_REPO,
            MAIN_BRANCH,
            README_FILE,
            &readme_content,
            "Initialize profile",
            name,
            email,
        )
        .await?;
    } else {
        // Update existing branch
        add_file_to_branch(
            state,
            user_slug,
            PROFILE_REPO,
            MAIN_BRANCH,
            README_FILE,
            &readme_content,
            "Update profile",
            name,
            email,
        )
        .await?;
    }

    Ok(())
}

/// Save SSH keys to .profile/authorized_keys
pub async fn save_ssh_keys(
    state: &GlobalState,
    user_slug: &str,
    keys: &[String],
    author_name: &str,
    author_email: &str,
) -> Result<()> {
    // Ensure .profile repo exists
    if !profile_repo_exists(state, user_slug).await {
        return Err(anyhow!(".profile repo does not exist"));
    }

    // Generate authorized_keys content
    let content = if keys.is_empty() {
        String::new()
    } else {
        keys.join("\n") + "\n"
    };

    // Check if main branch exists
    if !branch_exists(state, user_slug, PROFILE_REPO, MAIN_BRANCH).await {
        // Create orphan branch with authorized_keys
        create_orphan_branch(
            state,
            user_slug,
            PROFILE_REPO,
            MAIN_BRANCH,
            SSH_KEYS_FILE,
            &content,
            "Initialize SSH keys",
            author_name,
            author_email,
        )
        .await?;
    } else {
        // Update existing branch
        add_file_to_branch(
            state,
            user_slug,
            PROFILE_REPO,
            MAIN_BRANCH,
            SSH_KEYS_FILE,
            &content,
            "Update SSH keys",
            author_name,
            author_email,
        )
        .await?;
    }

    Ok(())
}

/// Create the .profile repository for a user
pub async fn create_profile_repo(
    state: &GlobalState,
    user_slug: &str,
    name: &str,
    email: &str,
    website: &str,
    default_main_branch: &str,
    description: &str,
    created_at: OffsetDateTime,
) -> Result<()> {
    use crate::services::repository::create_bare_repo;

    // Create bare repo
    create_bare_repo(state, user_slug.to_string(), PROFILE_REPO.to_string())
        .await
        .map_err(|e| anyhow!("Failed to create .profile repo: {}", e))?;

    // Create main branch with README.md
    let readme_content = generate_readme(
        name,
        email,
        website,
        default_main_branch,
        description,
        created_at,
    );
    create_orphan_branch(
        state,
        user_slug,
        PROFILE_REPO,
        MAIN_BRANCH,
        README_FILE,
        &readme_content,
        "Initialize profile",
        name,
        email,
    )
    .await?;

    // Add empty authorized_keys file
    add_file_to_branch(
        state,
        user_slug,
        PROFILE_REPO,
        MAIN_BRANCH,
        SSH_KEYS_FILE,
        "",
        "Initialize SSH keys",
        name,
        email,
    )
    .await?;

    Ok(())
}

/// Validate all SSH keys and return invalid ones
pub fn find_invalid_ssh_keys(keys: &[String]) -> Vec<String> {
    keys.iter()
        .filter(|key| !key.is_empty() && ssh_key::PublicKey::from_openssh(key).is_err())
        .cloned()
        .collect()
}

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Output;

use super::{Api, GitHelper, KeyType, TestSshKey};

/// A test user with SSH key and API client configured.
///
/// This bundles together common test setup:
/// - SSH key generation
/// - User registration
/// - Settings update with SSH key
pub struct TestUser {
    pub username: String,
    pub email: String,
    pub ssh_key: TestSshKey,
    ssh_public_host: String,
}

impl TestUser {
    /// Create and register a new test user with an Ed25519 SSH key.
    pub async fn create(
        api: &Api,
        temp_dir: &Path,
        username: &str,
        ssh_public_host: &str,
    ) -> Result<Self> {
        Self::create_with_key_type(api, temp_dir, username, ssh_public_host, KeyType::Ed25519).await
    }

    /// Create and register a new test user with a specific SSH key type.
    pub async fn create_with_key_type(
        api: &Api,
        temp_dir: &Path,
        username: &str,
        ssh_public_host: &str,
        key_type: KeyType,
    ) -> Result<Self> {
        let email = format!("{}@test.com", username);
        let ssh_key = TestSshKey::generate(temp_dir, username, key_type)?;

        api.register(username, &email, "password123456789").await?;
        api.update_settings(username, &email, "", "", "main", &ssh_key.public_key_content)
            .await?;

        Ok(Self {
            username: username.to_string(),
            email,
            ssh_key,
            ssh_public_host: ssh_public_host.to_string(),
        })
    }

    /// Build an SSH URL for a project owned by this user.
    pub fn ssh_url(&self, project_slug: &str) -> String {
        format!(
            "ssh://{}@{}/~{}/{}",
            self.username, self.ssh_public_host, self.username, project_slug
        )
    }

    /// Build an SSH URL for a project owned by another user.
    pub fn ssh_url_for(&self, owner: &str, project_slug: &str) -> String {
        format!(
            "ssh://{}@{}/~{}/{}",
            self.username, self.ssh_public_host, owner, project_slug
        )
    }

    /// Build an anonymous SSH URL for a project.
    pub fn anon_ssh_url(&self, owner: &str, project_slug: &str) -> String {
        format!(
            "ssh://anon@{}/~{}/{}",
            self.ssh_public_host, owner, project_slug
        )
    }

    /// Create a GitHelper configured for this user in the given work directory.
    pub fn git_helper(&self, work_dir: PathBuf) -> GitHelper {
        GitHelper::new(
            work_dir,
            &self.ssh_key.private_key_path,
            &capitalize(&self.username),
            &self.email,
        )
    }

    /// Create work directory and return a GitHelper for this user.
    pub fn setup_workspace(&self, temp_dir: &Path, name: &str) -> Result<(PathBuf, GitHelper)> {
        let work_dir = temp_dir.join(name);
        std::fs::create_dir_all(&work_dir)?;
        Ok((work_dir.clone(), self.git_helper(work_dir)))
    }
}

/// Helper to capitalize first letter of username for git identity.
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

/// Assertion helpers for git operations.
pub mod assertions {
    use std::process::Output;

    /// Assert that a git operation succeeded.
    pub fn assert_success(output: &Output, operation: &str) {
        assert!(
            output.status.success(),
            "{} failed: {}",
            operation,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// Assert that a git operation failed.
    pub fn assert_failure(output: &Output, operation: &str) {
        assert!(
            !output.status.success(),
            "{} should have failed but succeeded",
            operation
        );
    }

    /// Assert clone succeeded.
    pub fn assert_clone_success(output: &Output) {
        assert_success(output, "Clone");
    }

    /// Assert clone failed.
    pub fn assert_clone_failure(output: &Output) {
        assert_failure(output, "Clone");
    }

    /// Assert push succeeded.
    pub fn assert_push_success(output: &Output) {
        assert_success(output, "Push");
    }

    /// Assert push failed.
    pub fn assert_push_failure(output: &Output) {
        assert_failure(output, "Push");
    }
}

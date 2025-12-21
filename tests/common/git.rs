use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Output;
use tokio::process::Command;

/// Git command wrapper for tests.
///
/// Provides convenient methods for executing git commands with proper
/// SSH configuration for the test server.
///
/// IMPORTANT: All methods are async because tests run in single-threaded
/// Tokio runtime. Using std::process::Command would block the thread
/// and prevent the SSH server from processing connections.
pub struct GitHelper {
    /// Directory where git operations will be performed
    work_dir: PathBuf,
    /// SSH command with custom options for test server
    ssh_command: String,
    /// User identity for commits
    user_name: String,
    user_email: String,
}

impl GitHelper {
    /// Create a new GitHelper configured for the test SSH server.
    ///
    /// The SSH command is configured to:
    /// - Use the specified private key
    /// - Disable strict host key checking (test server generates new keys each run)
    /// - Use /dev/null as known_hosts file (ephemeral test)
    /// - Suppress SSH warnings
    pub fn new(
        work_dir: PathBuf,
        ssh_key_path: &Path,
        user_name: &str,
        user_email: &str,
    ) -> Self {
        let ssh_command = format!(
            "ssh -i {} -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR",
            ssh_key_path.display()
        );

        Self {
            work_dir,
            ssh_command,
            user_name: user_name.to_string(),
            user_email: user_email.to_string(),
        }
    }

    /// Clone a repository via SSH.
    ///
    /// The ssh_url should be in the format: `ssh://{user}@{host}/~{owner}/{project}`
    pub async fn clone_ssh(&self, ssh_url: &str, target_dir: &str) -> Result<Output> {
        let output = Command::new("git")
            .current_dir(&self.work_dir)
            .env("GIT_SSH_COMMAND", &self.ssh_command)
            .arg("clone")
            .arg(ssh_url)
            .arg(target_dir)
            .output()
            .await?;
        Ok(output)
    }

    /// Clone a repository via HTTP.
    pub async fn clone_http(&self, http_url: &str, target_dir: &str) -> Result<Output> {
        let output = Command::new("git")
            .current_dir(&self.work_dir)
            .arg("clone")
            .arg(http_url)
            .arg(target_dir)
            .output()
            .await?;
        Ok(output)
    }

    /// Initialize git config in a repo directory (user.name and user.email).
    pub async fn configure_identity(&self, repo_dir: &Path) -> Result<()> {
        Command::new("git")
            .current_dir(repo_dir)
            .args(["config", "user.name", &self.user_name])
            .status()
            .await?;
        Command::new("git")
            .current_dir(repo_dir)
            .args(["config", "user.email", &self.user_email])
            .status()
            .await?;
        Ok(())
    }

    /// Create a file, add it to git, and commit with the given message.
    pub async fn create_commit(
        &self,
        repo_dir: &Path,
        filename: &str,
        content: &str,
        message: &str,
    ) -> Result<Output> {
        // Write file
        std::fs::write(repo_dir.join(filename), content)?;

        // Add file
        Command::new("git")
            .current_dir(repo_dir)
            .args(["add", filename])
            .status()
            .await?;

        // Commit
        let output = Command::new("git")
            .current_dir(repo_dir)
            .args(["commit", "-m", message])
            .output()
            .await?;
        Ok(output)
    }

    /// Push to remote via SSH.
    pub async fn push_ssh(&self, repo_dir: &Path, remote: &str, branch: &str) -> Result<Output> {
        let output = Command::new("git")
            .current_dir(repo_dir)
            .env("GIT_SSH_COMMAND", &self.ssh_command)
            .args(["push", remote, branch])
            .output()
            .await?;
        Ok(output)
    }

    /// Fetch from remote via SSH.
    pub async fn fetch_ssh(&self, repo_dir: &Path, remote: &str) -> Result<Output> {
        let output = Command::new("git")
            .current_dir(repo_dir)
            .env("GIT_SSH_COMMAND", &self.ssh_command)
            .args(["fetch", remote])
            .output()
            .await?;
        Ok(output)
    }

    /// Pull from remote via SSH.
    pub async fn pull_ssh(&self, repo_dir: &Path, remote: &str, branch: &str) -> Result<Output> {
        let output = Command::new("git")
            .current_dir(repo_dir)
            .env("GIT_SSH_COMMAND", &self.ssh_command)
            .args(["pull", remote, branch])
            .output()
            .await?;
        Ok(output)
    }
}

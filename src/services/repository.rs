//! Repository service adapter
//!
//! This module wraps rubhub-repo functions, providing GlobalState-based interfaces
//! and handling event emission for repository changes.

use anyhow::Result;
use time::OffsetDateTime;

use crate::{
    GlobalState,
    models::{RepoEvent, RepoEventInfo},
};
use rubhub_auth_store::AuthStore;

// Re-export types from rubhub-repo
pub use rubhub_repo::{EntryKind, GitRefInfo, GitSummary, GitTreeEntry};

/// Newtype wrapper to implement AuthorResolver for AuthStore
struct AuthStoreResolver<'a>(&'a AuthStore);

impl rubhub_repo::AuthorResolver for AuthStoreResolver<'_> {
    fn resolve_email(&self, email: &str) -> Option<String> {
        self.0.get_user_slug_by_email(email)
    }
}

/// Parameters for creating a commit on a branch
pub struct CommitParams<'a> {
    pub state: &'a GlobalState,
    pub user_name: &'a str,
    pub project_slug: &'a str,
    pub branch_name: &'a str,
    pub file_path: &'a str,
    pub file_content: &'a str,
    pub commit_message: &'a str,
    pub author_name: &'a str,
    pub author_email: &'a str,
}

/// Create a new bare git repository
pub async fn create_bare_repo(
    state: &GlobalState,
    user: String,
    project: String,
) -> Result<(), std::io::Error> {
    rubhub_repo::create_bare_repo(&state.config.git_root, &user, &project)
        .await
        .map_err(|e| std::io::Error::other(e.to_string()))
}

/// Set the HEAD reference for a repository
pub async fn set_git_head(
    state: &GlobalState,
    user_name: &str,
    project_slug: &str,
    head_branch: &str,
) -> Result<()> {
    rubhub_repo::set_git_head(&state.config.git_root, user_name, project_slug, head_branch).await
}

/// Get a summary of a repository's branches and tags
pub async fn get_git_summary(
    state: &GlobalState,
    user_name: &str,
    project_slug: &str,
) -> Option<GitSummary> {
    rubhub_repo::get_git_summary(&state.config.git_root, user_name, project_slug).await
}

/// Get information about a git reference (branch/tag) with commits
pub async fn get_git_info(
    state: &GlobalState,
    user_name: &str,
    project_slug: &str,
    branch: &str,
    max_commits: i32,
    offset: i32,
) -> Option<GitRefInfo> {
    rubhub_repo::get_git_info(
        &state.config.git_root,
        user_name,
        project_slug,
        branch,
        max_commits,
        offset,
        &AuthStoreResolver(state.auth.as_ref()),
    )
    .await
}

/// Get the contents of a file from a repository
pub async fn get_git_file(
    state: &GlobalState,
    user_name: &str,
    project_slug: &str,
    branch: &str,
    path: &str,
) -> Result<Vec<u8>> {
    rubhub_repo::get_git_file(&state.config.git_root, user_name, project_slug, branch, path).await
}

/// Get the entries in a directory from a repository
pub async fn get_git_tree(
    state: &GlobalState,
    user_name: &str,
    project_slug: &str,
    branch: &str,
    path: &str,
) -> Result<Vec<GitTreeEntry>> {
    rubhub_repo::get_git_tree(&state.config.git_root, user_name, project_slug, branch, path).await
}

/// Check if a branch exists in a repository
pub async fn branch_exists(
    state: &GlobalState,
    user_name: &str,
    project_slug: &str,
    branch: &str,
) -> bool {
    rubhub_repo::branch_exists(&state.config.git_root, user_name, project_slug, branch).await
}

/// Emit events for repository changes
fn emit_changes(before: &GitSummary, after: &GitSummary, state: &GlobalState) {
    let timestamp = OffsetDateTime::now_utc();
    let (branch_changes, tag_changes) = before.diff(after);

    for (branch, commit_hash) in branch_changes {
        state.emit_event(RepoEvent::BranchUpdated {
            info: RepoEventInfo {
                owner: after.owner().to_string(),
                project: after.project().to_string(),
                commit_hash,
                timestamp,
            },
            branch,
        });
    }

    for (tag, commit_hash) in tag_changes {
        state.emit_event(RepoEvent::TagUpdated {
            info: RepoEventInfo {
                owner: after.owner().to_string(),
                project: after.project().to_string(),
                commit_hash,
                timestamp,
            },
            tag,
        });
    }
}

/// Create an orphan branch with an initial commit containing one file
pub async fn create_orphan_branch(params: CommitParams<'_>) -> Result<()> {
    let state = params.state;
    let git_root = &state.config.git_root;

    // Capture state before operation
    let before =
        rubhub_repo::capture_git_summary(git_root, params.user_name, params.project_slug);

    // Convert to rubhub_repo::CommitParams
    let repo_params = rubhub_repo::CommitParams {
        git_root,
        user_name: params.user_name,
        project_slug: params.project_slug,
        branch_name: params.branch_name,
        file_path: params.file_path,
        file_content: params.file_content,
        commit_message: params.commit_message,
        author_name: params.author_name,
        author_email: params.author_email,
    };

    rubhub_repo::create_orphan_branch(repo_params).await?;

    // Capture state after and emit changes
    let after = rubhub_repo::capture_git_summary(git_root, params.user_name, params.project_slug);
    emit_changes(&before, &after, state);

    Ok(())
}

/// Capture git summary for use with emit_repo_changes
pub fn capture_summary(state: &GlobalState, owner: &str, project: &str) -> GitSummary {
    rubhub_repo::capture_git_summary(&state.config.git_root, owner, project)
}

/// Emit repository change events by comparing before/after states
pub fn emit_repo_changes(before: &GitSummary, after: &GitSummary, state: &GlobalState) {
    emit_changes(before, after, state);
}

/// Add a file to a branch and create a commit
pub async fn add_file_to_branch(params: CommitParams<'_>) -> Result<()> {
    let state = params.state;
    let git_root = &state.config.git_root;

    // Capture state before operation
    let before =
        rubhub_repo::capture_git_summary(git_root, params.user_name, params.project_slug);

    // Convert to rubhub_repo::CommitParams
    let repo_params = rubhub_repo::CommitParams {
        git_root,
        user_name: params.user_name,
        project_slug: params.project_slug,
        branch_name: params.branch_name,
        file_path: params.file_path,
        file_content: params.file_content,
        commit_message: params.commit_message,
        author_name: params.author_name,
        author_email: params.author_email,
    };

    rubhub_repo::add_file_to_branch(repo_params).await?;

    // Capture state after and emit changes
    let after = rubhub_repo::capture_git_summary(git_root, params.user_name, params.project_slug);
    emit_changes(&before, &after, state);

    Ok(())
}

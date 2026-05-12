//! Git repository operations for RubHub
//!
//! This crate provides a clean abstraction over git repository operations,
//! hiding the underlying gix implementation from consumers.

mod entry_kind;
mod error;
mod repository;
mod types;

// Re-export public types
pub use entry_kind::EntryKind;
pub use error::RepoError;
pub use types::{
    AuthorResolver, CommitParams, GitCommitInfo, GitRefInfo, GitSummary, GitTreeEntry,
    format_relative_time,
};

// Re-export repository functions
pub use repository::{
    add_file_to_branch, branch_exists, capture_git_summary, create_bare_repo, create_orphan_branch,
    fork_bare_repo, get_git_file, get_git_info, get_git_summary, get_git_tree, set_git_head,
};

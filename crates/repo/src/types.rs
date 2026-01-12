use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::Path;
use time::OffsetDateTime;

use crate::entry_kind::EntryKind;

/// Trait for resolving author emails to user slugs
///
/// Implement this trait to enable author linking in commit info.
/// The main crate can implement this for AuthStore.
pub trait AuthorResolver: Send + Sync {
    fn resolve_email(&self, email: &str) -> Option<String>;
}

/// No-op implementation for when author resolution is not needed
impl AuthorResolver for () {
    fn resolve_email(&self, _email: &str) -> Option<String> {
        None
    }
}

/// Parameters for creating a commit on a branch
pub struct CommitParams<'a> {
    pub git_root: &'a Path,
    pub user_name: &'a str,
    pub project_slug: &'a str,
    pub branch_name: &'a str,
    pub file_path: &'a str,
    pub file_content: &'a str,
    pub commit_message: &'a str,
    pub author_name: &'a str,
    pub author_email: &'a str,
}

/// An entry in a git tree (directory listing)
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GitTreeEntry {
    pub filename: String,
    pub kind: EntryKind,
}

impl GitTreeEntry {
    /// Get the full path of this entry given the current directory path
    pub fn full_path(&self, current_path: &str) -> String {
        if current_path.is_empty() {
            self.filename.clone()
        } else {
            format!("{}/{}", current_path, self.filename)
        }
    }

    /// Generate the URI for viewing this entry as a tree (directory)
    pub fn uri_tree(
        &self,
        project_owner: &str,
        project_slug: &str,
        git_ref: &str,
        current_path: &str,
    ) -> String {
        let full_path = self.full_path(current_path);
        let encoded_ref = urlencoding::encode(git_ref);
        format!(
            "/~{}/{}/tree/{}/{}",
            project_owner, project_slug, encoded_ref, full_path
        )
    }

    /// Generate the URI for viewing this entry as a blob (file)
    pub fn uri_blob(
        &self,
        project_owner: &str,
        project_slug: &str,
        git_ref: &str,
        current_path: &str,
    ) -> String {
        let full_path = self.full_path(current_path);
        let encoded_ref = urlencoding::encode(git_ref);
        format!(
            "/~{}/{}/blob/{}/{}",
            project_owner, project_slug, encoded_ref, full_path
        )
    }

    /// Returns true if this entry is a directory
    pub fn is_directory(&self) -> bool {
        self.kind.is_tree()
    }

    /// Returns true if this entry is a file
    pub fn is_file(&self) -> bool {
        self.kind.is_blob()
    }
}

impl Ord for GitTreeEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Sort directories before files, then alphabetically by name
        match (self.kind, other.kind) {
            (EntryKind::Tree, EntryKind::Tree) => self.filename.cmp(&other.filename),
            (EntryKind::Tree, _) => Ordering::Less,
            (_, EntryKind::Tree) => Ordering::Greater,
            _ => self.filename.cmp(&other.filename),
        }
    }
}

impl PartialOrd for GitTreeEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Information about a single commit
#[derive(Debug, Clone)]
pub struct GitCommitInfo {
    pub id: String,
    pub author: String,
    pub author_user_slug: Option<String>,
    pub message: String,
    pub time: OffsetDateTime,
}

impl GitCommitInfo {
    /// Format the commit time as a human-readable relative time string
    pub fn relative_time(&self) -> String {
        let now = OffsetDateTime::now_utc();
        let diff = (now - self.time).whole_seconds();
        format_relative_time(diff)
    }
}

/// Information about a git reference (branch/tag) and its commits
#[derive(Debug, Clone)]
pub struct GitRefInfo {
    pub branch_name: String,
    pub commit_count: i32,
    pub commits: Vec<GitCommitInfo>,
}

/// Summary of a repository's refs state, used for both display and change detection
#[derive(Debug, Clone)]
pub struct GitSummary {
    owner: String,
    project: String,
    branches: HashMap<String, String>,
    tags: HashMap<String, String>,
}

impl GitSummary {
    /// Create a new GitSummary with the given data
    pub(crate) fn new(
        owner: String,
        project: String,
        branches: HashMap<String, String>,
        tags: HashMap<String, String>,
    ) -> Self {
        Self {
            owner,
            project,
            branches,
            tags,
        }
    }

    /// Get the owner slug
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// Get the project slug
    pub fn project(&self) -> &str {
        &self.project
    }

    /// Get branch names as a sorted vec
    pub fn branches(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.branches.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    /// Get tag names as a sorted vec
    pub fn tags(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.tags.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    /// Get the raw branches map (for diffing)
    pub fn branches_raw(&self) -> &HashMap<String, String> {
        &self.branches
    }

    /// Get the raw tags map (for diffing)
    pub fn tags_raw(&self) -> &HashMap<String, String> {
        &self.tags
    }

    /// Compare with another state and return the changes
    ///
    /// Returns (new/updated branches, new/updated tags) as Vec of (name, commit_hash)
    pub fn diff(&self, after: &GitSummary) -> (Vec<(String, String)>, Vec<(String, String)>) {
        let mut branch_changes = Vec::new();
        let mut tag_changes = Vec::new();

        // Check for new or updated branches
        for (branch, new_hash) in &after.branches {
            if self.branches.get(branch) != Some(new_hash) {
                branch_changes.push((branch.clone(), new_hash.clone()));
            }
        }

        // Check for new or updated tags
        for (tag, new_hash) in &after.tags {
            if self.tags.get(tag) != Some(new_hash) {
                tag_changes.push((tag.clone(), new_hash.clone()));
            }
        }

        (branch_changes, tag_changes)
    }
}

/// Format a time difference in seconds as a human-readable relative time string
pub fn format_relative_time(seconds: i64) -> String {
    if seconds < 60 {
        return format!(
            "{} second{} ago",
            seconds,
            if seconds != 1 { "s" } else { "" }
        );
    }
    if seconds < 3600 {
        let minutes = seconds / 60;
        return format!(
            "{} minute{} ago",
            minutes,
            if minutes != 1 { "s" } else { "" }
        );
    }
    if seconds < 86400 {
        let hours = seconds / 3600;
        return format!("{} hour{} ago", hours, if hours != 1 { "s" } else { "" });
    }
    if seconds < 86400 * 30 {
        let days = seconds / 86400;
        return format!("{} day{} ago", days, if days != 1 { "s" } else { "" });
    }
    if seconds < 86400 * 365 {
        let months = seconds / (86400 * 30);
        return format!(
            "{} month{} ago",
            months,
            if months != 1 { "s" } else { "" }
        );
    }
    let years = seconds / (86400 * 365);
    format!("{} year{} ago", years, if years != 1 { "s" } else { "" })
}

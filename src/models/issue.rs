use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::common::format_relative_time;

/// Status of an issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum IssueStatus {
    #[default]
    Open,
    Completed,
    Cancelled,
}

impl IssueStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Frontmatter parsed from issue comment files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentFrontmatter {
    #[serde(with = "time::serde::rfc3339")]
    pub date: OffsetDateTime,
    pub author: String,
    pub email: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub status: Option<IssueStatus>,
}

/// A single comment on an issue
#[derive(Debug, Clone)]
pub struct IssueComment {
    pub date: OffsetDateTime,
    pub author: String,
    pub content_html: String,
    pub status_change: Option<IssueStatus>,
}

/// An issue with all its comments
#[derive(Debug, Clone)]
pub struct Issue {
    pub dir_name: String,
    pub title: String,
    pub status: IssueStatus,
    pub comments: Vec<IssueComment>,
}

/// Summary for list view (without loading all comments)
#[derive(Debug, Clone)]
pub struct IssueSummary {
    pub dir_name: String,
    pub title: String,
    pub created_at: OffsetDateTime,
    pub author: String,
    pub status: IssueStatus,
    pub comment_count: usize,
}

impl Issue {
    /// Determine current status from comments (last status-changing comment wins)
    pub fn compute_status(comments: &[IssueComment]) -> IssueStatus {
        comments
            .iter()
            .rev()
            .find_map(|c| c.status_change)
            .unwrap_or_default()
    }
}

impl IssueSummary {
    /// URI for viewing this issue
    pub fn uri(&self, owner: &str, project_slug: &str) -> String {
        format!("/~{}/{}/issues/{}", owner, project_slug, self.dir_name)
    }

    /// Format the created_at date for display
    pub fn relative_time(&self) -> String {
        let now = OffsetDateTime::now_utc();
        let diff = now - self.created_at;
        format_relative_time(diff.whole_seconds())
    }
}

impl IssueComment {
    /// Format the date for display
    pub fn relative_time(&self) -> String {
        let now = OffsetDateTime::now_utc();
        let diff = now - self.date;
        format_relative_time(diff.whole_seconds())
    }
}

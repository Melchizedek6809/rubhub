use serde::Serialize;
use time::OffsetDateTime;

/// Common fields for all repository events
#[derive(Debug, Clone, Serialize)]
pub struct RepoEventInfo {
    pub owner: String,
    pub project: String,
    pub commit_hash: String,
    #[serde(with = "time::serde::rfc3339")]
    pub timestamp: OffsetDateTime,
}

/// Events emitted when repository state changes
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "T")]
pub enum RepoEvent {
    BranchUpdated {
        #[serde(flatten)]
        info: RepoEventInfo,
        branch: String,
    },
    TagUpdated {
        #[serde(flatten)]
        info: RepoEventInfo,
        tag: String,
    },
}

impl RepoEvent {
    pub fn info(&self) -> &RepoEventInfo {
        match self {
            RepoEvent::BranchUpdated { info, .. } => info,
            RepoEvent::TagUpdated { info, .. } => info,
        }
    }
}

use serde::Serialize;
use time::OffsetDateTime;

use super::AccessType;

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
    RepositoryCreated {
        owner: String,
        project: String,
        public_access: AccessType,
        #[serde(with = "time::serde::rfc3339")]
        timestamp: OffsetDateTime,
    },
    RepositoryDeleted {
        owner: String,
        project: String,
        public_access: AccessType,
        #[serde(with = "time::serde::rfc3339")]
        timestamp: OffsetDateTime,
    },
}

impl RepoEvent {
    /// Returns the owner of the repository this event relates to.
    pub fn owner(&self) -> &str {
        match self {
            RepoEvent::BranchUpdated { info, .. } => &info.owner,
            RepoEvent::TagUpdated { info, .. } => &info.owner,
            RepoEvent::RepositoryCreated { owner, .. } => owner,
            RepoEvent::RepositoryDeleted { owner, .. } => owner,
        }
    }

    /// Returns the project slug this event relates to.
    pub fn project(&self) -> &str {
        match self {
            RepoEvent::BranchUpdated { info, .. } => &info.project,
            RepoEvent::TagUpdated { info, .. } => &info.project,
            RepoEvent::RepositoryCreated { project, .. } => project,
            RepoEvent::RepositoryDeleted { project, .. } => project,
        }
    }

    /// Returns true if this is a repository deletion event.
    pub fn is_repository_deleted(&self) -> bool {
        matches!(self, RepoEvent::RepositoryDeleted { .. })
    }

    /// Returns true if this is a repository creation event.
    pub fn is_repository_created(&self) -> bool {
        matches!(self, RepoEvent::RepositoryCreated { .. })
    }

    /// Returns the public access level for create/delete events (for access control filtering).
    /// These events carry their own access info since the project may not be loadable.
    pub fn event_public_access(&self) -> Option<AccessType> {
        match self {
            RepoEvent::RepositoryCreated { public_access, .. } => Some(*public_access),
            RepoEvent::RepositoryDeleted { public_access, .. } => Some(*public_access),
            _ => None,
        }
    }
}

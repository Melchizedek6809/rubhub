use std::sync::mpsc::SendError;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{event::StoreEvent, AuthStore};

/// Access level for public users
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PublicAccess {
    #[default]
    None,
    Read,
    Write,
}

impl PublicAccess {
    pub fn as_str(&self) -> &'static str {
        match self {
            PublicAccess::None => "none",
            PublicAccess::Read => "read",
            PublicAccess::Write => "write",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "none" => Some(PublicAccess::None),
            "read" => Some(PublicAccess::Read),
            "write" => Some(PublicAccess::Write),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectInfo {
    /// Key format: "~owner/slug" for user projects
    pub key: String,

    /// Human-readable project name
    pub name: String,

    /// Short description
    pub description: String,

    /// Default branch name (e.g., "main", "master")
    pub default_branch: String,

    /// Public access level
    pub public_access: PublicAccess,

    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl ProjectInfo {
    /// Create a new ProjectInfo with the composite key
    pub fn new(
        owner: &str,
        slug: &str,
        name: String,
        description: String,
        default_branch: String,
        public_access: PublicAccess,
    ) -> Self {
        Self {
            key: Self::make_key(owner, slug),
            name,
            description,
            default_branch,
            public_access,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    /// Generate the composite key from owner and slug
    pub fn make_key(owner: &str, slug: &str) -> String {
        format!("~{}/{}", owner, slug)
    }

    /// Parse owner and slug from the key
    pub fn parse_key(key: &str) -> Option<(&str, &str)> {
        let key = key.strip_prefix('~')?;
        let (owner, slug) = key.split_once('/')?;
        Some((owner, slug))
    }

    /// Get owner from this project's key
    pub fn owner(&self) -> Option<&str> {
        Self::parse_key(&self.key).map(|(owner, _)| owner)
    }

    /// Get slug from this project's key
    pub fn slug(&self) -> Option<&str> {
        Self::parse_key(&self.key).map(|(_, slug)| slug)
    }

    pub fn save(self, store: &AuthStore) -> Result<(), SendError<StoreEvent>> {
        store.store_event(StoreEvent::ProjectInfo(self))
    }

    pub fn delete(store: &AuthStore, key: String) -> Result<(), SendError<StoreEvent>> {
        store.store_event(StoreEvent::ProjectInfoDelete { key })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_key() {
        assert_eq!(ProjectInfo::make_key("alice", "my-project"), "~alice/my-project");
        assert_eq!(ProjectInfo::make_key("bob", "test"), "~bob/test");
    }

    #[test]
    fn test_parse_key() {
        assert_eq!(ProjectInfo::parse_key("~alice/my-project"), Some(("alice", "my-project")));
        assert_eq!(ProjectInfo::parse_key("~bob/test"), Some(("bob", "test")));
        assert_eq!(ProjectInfo::parse_key("alice/my-project"), None); // missing ~
        assert_eq!(ProjectInfo::parse_key("~alice"), None); // missing slug
        assert_eq!(ProjectInfo::parse_key("invalid"), None);
    }

    #[test]
    fn test_public_access_as_str() {
        assert_eq!(PublicAccess::None.as_str(), "none");
        assert_eq!(PublicAccess::Read.as_str(), "read");
        assert_eq!(PublicAccess::Write.as_str(), "write");
    }

    #[test]
    fn test_public_access_from_str() {
        assert_eq!(PublicAccess::from_str("none"), Some(PublicAccess::None));
        assert_eq!(PublicAccess::from_str("read"), Some(PublicAccess::Read));
        assert_eq!(PublicAccess::from_str("write"), Some(PublicAccess::Write));
        assert_eq!(PublicAccess::from_str("READ"), Some(PublicAccess::Read)); // case insensitive
        assert_eq!(PublicAccess::from_str("invalid"), None);
    }

    #[test]
    fn test_project_info_owner_slug() {
        let info = ProjectInfo::new(
            "alice",
            "my-project",
            "My Project".to_string(),
            "A test project".to_string(),
            "main".to_string(),
            PublicAccess::Read,
        );
        assert_eq!(info.owner(), Some("alice"));
        assert_eq!(info.slug(), Some("my-project"));
        assert_eq!(info.key, "~alice/my-project");
    }

    #[test]
    fn test_project_info_save_load_delete() {
        use tempfile::tempdir;
        use crate::AuthStore;

        // Create a temp directory for the auth store
        let dir = tempdir().unwrap();
        let store = AuthStore::new(dir.path().to_path_buf());

        // Create and save a project
        let info = ProjectInfo::new(
            "alice",
            "test-project",
            "Test Project".to_string(),
            "A test project".to_string(),
            "main".to_string(),
            PublicAccess::Read,
        );
        info.save(&store).unwrap();

        // Verify it can be loaded
        let loaded = store.get_project_by_owner_slug("alice", "test-project");
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.name, "Test Project");
        assert_eq!(loaded.description, "A test project");
        assert_eq!(loaded.default_branch, "main");
        assert_eq!(loaded.public_access, PublicAccess::Read);

        // Verify it appears in owner's projects
        let owner_projects = store.get_projects_for_owner("alice");
        assert_eq!(owner_projects.len(), 1);
        assert_eq!(owner_projects[0].key, "~alice/test-project");

        // Verify it appears in public projects
        let public_projects = store.get_public_projects();
        assert_eq!(public_projects.len(), 1);

        // Delete the project
        ProjectInfo::delete(&store, "~alice/test-project".to_string()).unwrap();

        // Verify it's gone
        assert!(store.get_project_by_owner_slug("alice", "test-project").is_none());
        assert!(store.get_projects_for_owner("alice").is_empty());
        assert!(store.get_public_projects().is_empty());
    }

    #[test]
    fn test_project_info_private_not_in_public_list() {
        use tempfile::tempdir;
        use crate::AuthStore;

        let dir = tempdir().unwrap();
        let store = AuthStore::new(dir.path().to_path_buf());

        // Create a private project
        let info = ProjectInfo::new(
            "bob",
            "private-project",
            "Private Project".to_string(),
            "A private project".to_string(),
            "main".to_string(),
            PublicAccess::None,
        );
        info.save(&store).unwrap();

        // Should exist in owner's list
        let owner_projects = store.get_projects_for_owner("bob");
        assert_eq!(owner_projects.len(), 1);

        // Should NOT appear in public projects
        let public_projects = store.get_public_projects();
        assert!(public_projects.is_empty());
    }

    #[test]
    fn test_project_info_update() {
        use tempfile::tempdir;
        use crate::AuthStore;

        let dir = tempdir().unwrap();
        let store = AuthStore::new(dir.path().to_path_buf());

        // Create a project
        let info = ProjectInfo::new(
            "alice",
            "my-project",
            "Original Name".to_string(),
            "Original description".to_string(),
            "main".to_string(),
            PublicAccess::None,
        );
        info.save(&store).unwrap();

        // Update the project
        let updated = ProjectInfo {
            key: ProjectInfo::make_key("alice", "my-project"),
            name: "Updated Name".to_string(),
            description: "Updated description".to_string(),
            default_branch: "develop".to_string(),
            public_access: PublicAccess::Read,
            created_at: time::OffsetDateTime::now_utc(),
        };
        updated.save(&store).unwrap();

        // Verify the update
        let loaded = store.get_project_by_owner_slug("alice", "my-project").unwrap();
        assert_eq!(loaded.name, "Updated Name");
        assert_eq!(loaded.description, "Updated description");
        assert_eq!(loaded.default_branch, "develop");
        assert_eq!(loaded.public_access, PublicAccess::Read);

        // Should only have one project in owner's list (no duplicates)
        let owner_projects = store.get_projects_for_owner("alice");
        assert_eq!(owner_projects.len(), 1);
    }
}

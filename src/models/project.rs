use std::sync::Arc;

use anyhow::{Result, anyhow};
use rubhub_auth_store::{ProjectInfo, PublicAccess};
use time::OffsetDateTime;

use crate::{
    AccessType, GlobalState, User,
    services::{
        repository,
        validation::{slugify, validate_slug},
    },
};

#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    pub slug: String,
    pub owner: String,
    pub created_at: OffsetDateTime,
    pub public_access: AccessType,

    pub name: String,
    pub description: String,

    pub main_branch: String,
}

impl Project {
    /// Convert auth_store PublicAccess to AccessType
    fn public_access_to_access_type(pa: PublicAccess) -> AccessType {
        match pa {
            PublicAccess::None => AccessType::None,
            PublicAccess::Read => AccessType::Read,
            PublicAccess::Write => AccessType::Write,
        }
    }

    /// Convert AccessType to auth_store PublicAccess
    fn access_type_to_public_access(at: AccessType) -> PublicAccess {
        match at {
            AccessType::None => PublicAccess::None,
            AccessType::Read => PublicAccess::Read,
            AccessType::Write => PublicAccess::Write,
            AccessType::Admin => PublicAccess::Write, // Admin is internal only
        }
    }

    /// Create Project from ProjectInfo (for list views)
    pub fn from_project_info(info: &ProjectInfo) -> Option<Self> {
        let (owner, slug) = ProjectInfo::parse_key(&info.key)?;
        Some(Self {
            slug: slug.to_string(),
            owner: owner.to_string(),
            created_at: info.created_at,
            public_access: Self::public_access_to_access_type(info.public_access),
            name: info.name.clone(),
            description: info.description.clone(),
            main_branch: info.default_branch.clone(),
        })
    }

    /// Convert to ProjectInfo for auth_store
    pub fn to_project_info(&self) -> ProjectInfo {
        ProjectInfo {
            key: ProjectInfo::make_key(&self.owner, &self.slug),
            name: self.name.clone(),
            description: self.description.clone(),
            default_branch: self.main_branch.clone(),
            public_access: Self::access_type_to_public_access(self.public_access),
            created_at: self.created_at,
        }
    }

    pub async fn load(state: &GlobalState, user_slug: &str, project_slug: &str) -> Result<Self> {
        if validate_slug(user_slug).is_err() {
            return Err(anyhow!("Invalid username"));
        }
        if validate_slug(project_slug).is_err() {
            return Err(anyhow!("Invalid projectname"));
        }

        // Check if repo directory exists (this confirms project exists)
        let repo_path = state.config.git_root.join(user_slug).join(project_slug);
        if !tokio::fs::metadata(&repo_path)
            .await
            .map(|m| m.is_dir())
            .unwrap_or(false)
        {
            return Err(anyhow!("Project not found"));
        }

        // Load from auth_store
        let info = state
            .auth
            .get_project_by_owner_slug(user_slug, project_slug)
            .ok_or_else(|| anyhow!("Project not found in auth_store"))?;

        Self::from_project_info(&info).ok_or_else(|| anyhow!("Invalid project info"))
    }

    pub async fn save(&self, state: &GlobalState) -> Result<()> {
        if validate_slug(&self.owner).is_err() {
            return Err(anyhow!("Invalid username"));
        }
        if validate_slug(&self.slug).is_err() {
            return Err(anyhow!("Invalid projectname"));
        }

        // Save to auth_store (source of truth)
        self.to_project_info()
            .save(&state.auth)
            .map_err(|e| anyhow!("Failed to save project info: {}", e))?;

        // Update HEAD in Git
        repository::set_git_head(state, &self.owner, &self.slug, &self.main_branch).await?;

        Ok(())
    }

    pub async fn delete(&self, state: &GlobalState) -> Result<()> {
        // Validate slugs for safety (defense in depth)
        if validate_slug(&self.owner).is_err() {
            return Err(anyhow!("Invalid username"));
        }
        if validate_slug(&self.slug).is_err() {
            return Err(anyhow!("Invalid projectname"));
        }

        // Remove from auth_store
        let key = ProjectInfo::make_key(&self.owner, &self.slug);
        ProjectInfo::delete(&state.auth, key)
            .map_err(|e| anyhow!("Failed to delete project info: {}", e))?;

        // Delete the git repository directory
        let repo_path = state.config.git_root.join(&self.owner).join(&self.slug);
        tokio::fs::remove_dir_all(&repo_path).await?;

        Ok(())
    }

    pub fn new(user: &User, name: &str, public_access: AccessType) -> Result<Self> {
        let slug = slugify(name);
        if validate_slug(&user.slug).is_err() {
            return Err(anyhow!("Invalid username"));
        }
        if validate_slug(&slug).is_err() {
            return Err(anyhow!("Invalid projectname"));
        }

        Ok(Self {
            slug,
            created_at: time::OffsetDateTime::now_utc(),
            name: name.to_string(),
            description: String::new(),
            owner: user.slug.to_string(),
            public_access,
            main_branch: user.default_main_branch.to_string(),
        })
    }

    pub async fn access_level(&self, user_slug: Option<String>) -> AccessType {
        if let Some(user_slug) = user_slug
            && user_slug == self.owner
        {
            AccessType::Admin
        } else {
            self.public_access
        }
    }

    pub fn uri(&self) -> String {
        format!("/~{}/{}", self.owner, self.slug)
    }

    pub fn uri_settings(&self) -> String {
        format!("/~{}/{}/settings", self.owner, self.slug)
    }

    pub fn uri_branches(&self) -> String {
        format!("/~{}/{}/branches", self.owner, self.slug)
    }

    pub fn uri_tags(&self) -> String {
        format!("/~{}/{}/tags", self.owner, self.slug)
    }

    pub fn uri_talk(&self) -> String {
        format!("/~{}/{}/talk", self.owner, self.slug)
    }

    pub fn ssh_clone_url(&self, ssh_public_host: &str) -> String {
        format!(
            "ssh://git@{}/~{}/{}",
            ssh_public_host, self.owner, self.slug
        )
    }

    pub fn http_clone_url(&self, base_url: &str) -> String {
        format!("{}/~{}/{}", base_url, self.owner, self.slug)
    }

    pub fn uri_log(&self, branch: &str, page: i32) -> String {
        let encoded_branch = urlencoding::encode(branch);
        if page > 0 {
            format!(
                "/~{}/{}/log/{}?page={}",
                self.owner, self.slug, encoded_branch, page
            )
        } else {
            format!("/~{}/{}/log/{}", self.owner, self.slug, encoded_branch)
        }
    }

    pub fn uri_tree(&self, git_ref: &str, path: &str) -> String {
        let encoded_ref = urlencoding::encode(git_ref);
        if path.is_empty() {
            format!("/~{}/{}/tree/{}", self.owner, self.slug, encoded_ref)
        } else {
            format!(
                "/~{}/{}/tree/{}/{}",
                self.owner, self.slug, encoded_ref, path
            )
        }
    }

    pub fn uri_blob(&self, git_ref: &str, path: &str) -> String {
        let encoded_ref = urlencoding::encode(git_ref);
        format!(
            "/~{}/{}/blob/{}/{}",
            self.owner, self.slug, encoded_ref, path
        )
    }

    pub async fn load_by_path(state: &GlobalState, path: String) -> Result<(Arc<User>, Project)> {
        let parts = path.split("/").collect::<Vec<&str>>();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid path"));
        };
        let user_slug = parts[0];
        let project_slug = parts[1];
        if validate_slug(user_slug).is_err() {
            return Err(anyhow!("Invalid user"));
        }
        if validate_slug(project_slug).is_err() {
            return Err(anyhow!("Invalid project"));
        }

        if let Some(user) = state.auth.get_user(user_slug) {
            let project = Self::load(state, user_slug, project_slug).await?;
            Ok((user, project))
        } else {
            Err(anyhow!("Cant load user"))
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProjectSummary<'a> {
    pub name: &'a str,
    pub slug: &'a str,
    pub owner_slug: &'a str,
    pub owner_name: &'a str,
    pub description: &'a str,
}

impl<'a> ProjectSummary<'a> {
    pub fn uri(&self) -> String {
        format!("/~{}/{}", self.owner_slug, self.slug)
    }

    pub fn owner_uri(&self) -> String {
        format!("/~{}", self.owner_slug)
    }
}

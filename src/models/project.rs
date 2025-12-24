use anyhow::{Result, anyhow};
use time::OffsetDateTime;

use crate::{
    AccessType, GlobalState, User,
    services::{
        project_info::{self, load_project_info},
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
    pub website: String,

    pub main_branch: String,
}

impl Project {
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

        // Try to load from rubhub/info branch
        let (frontmatter, description) = load_project_info(state, user_slug, project_slug).await?;

        // Apply defaults for missing fields
        let name = frontmatter.name.unwrap_or_else(|| project_slug.to_string());

        let public_access = frontmatter
            .public_access
            .as_ref()
            .and_then(|s| AccessType::parse_public_access(s).ok())
            .unwrap_or(AccessType::None);

        let main_branch = match frontmatter.default_branch {
            Some(branch) => branch,
            None => project_info::detect_default_branch(state, user_slug, project_slug).await,
        };

        let website = frontmatter.website.unwrap_or_default();
        let created_at = frontmatter
            .created_at
            .unwrap_or_else(OffsetDateTime::now_utc);

        Ok(Self {
            slug: project_slug.to_string(),
            owner: user_slug.to_string(),
            created_at,
            public_access,
            name,
            description,
            website,
            main_branch,
        })
    }

    pub async fn save(
        &self,
        state: &GlobalState,
        author_name: &str,
        author_email: &str,
    ) -> Result<()> {
        if validate_slug(&self.owner).is_err() {
            return Err(anyhow!("Invalid username"));
        }
        if validate_slug(&self.slug).is_err() {
            return Err(anyhow!("Invalid projectname"));
        }

        // Save to rubhub/info branch
        project_info::save_project_info(
            state,
            &self.owner,
            &self.slug,
            self,
            author_name,
            author_email,
        )
        .await
    }

    pub async fn delete(&self, state: &GlobalState) -> Result<()> {
        // Validate slugs for safety (defense in depth)
        if validate_slug(&self.owner).is_err() {
            return Err(anyhow!("Invalid username"));
        }
        if validate_slug(&self.slug).is_err() {
            return Err(anyhow!("Invalid projectname"));
        }

        // Delete the git repository directory (metadata is stored within)
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
            description: "".to_string(),
            website: "".to_string(),
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

    pub fn uri_issues(&self) -> String {
        format!("/~{}/{}/issues", self.owner, self.slug)
    }

    pub fn ssh_clone_url(&self, ssh_public_host: &str, git_user: &str) -> String {
        format!(
            "ssh://{}@{}/~{}/{}",
            git_user, ssh_public_host, self.owner, self.slug
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

    pub async fn load_by_path(state: &GlobalState, path: String) -> Result<(User, Project)> {
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

        let user = User::load(state, user_slug).await?;
        let project = Self::load(state, user_slug, project_slug).await?;
        Ok((user, project))
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

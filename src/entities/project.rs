use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{
    AccessType, GlobalState, User,
    services::validation::{slugify, validate_slug},
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
        let filename = format!("!{project_slug}.json");
        let path = state.config.git_root.join(user_slug).join(filename);
        let data = tokio::fs::read(path).await?;
        let data = String::from_utf8_lossy(&data);
        let user: Project = serde_json::from_str(&data)?;

        Ok(user)
    }

    // ToDo: would be better to do things atomic, should be good enough for now though
    pub async fn save(&self, state: &GlobalState) -> Result<()> {
        if validate_slug(&self.owner).is_err() {
            return Err(anyhow!("Invalid username"));
        }
        if validate_slug(&self.slug).is_err() {
            return Err(anyhow!("Invalid projectname"));
        }
        let _user = User::load(state, &self.owner).await?;

        let path = state.config.git_root.join(&self.owner);
        tokio::fs::create_dir_all(&path).await?;
        let filename = format!("!{}.json", self.slug);
        let path = path.join(filename);
        let data = serde_json::to_string(&self)?;
        tokio::fs::write(path, data).await?;

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

    pub fn uri_log(&self, branch: &str, page: i32) -> String {
        if page > 0 {
            format!(
                "/~{}/{}/log/{}?page={}",
                self.owner, self.slug, branch, page
            )
        } else {
            format!("/~{}/{}/log/{}", self.owner, self.slug, branch)
        }
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

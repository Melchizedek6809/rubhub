use anyhow::{Result, anyhow};
use russh::keys::ssh_key;
use serde::{Deserialize, Serialize};

use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    GlobalState, Project,
    services::{
        password::{PasswordVerification, hash_password, verify_password_hash},
        validation::{slugify, validate_username},
    },
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub created_at: OffsetDateTime,
    pub last_login: Option<OffsetDateTime>,

    pub slug: String,
    pub name: String,
    pub email: String,
    pub description: String,
    pub website: String,

    pub default_main_branch: String,

    pub password_hash: String,
    pub ssh_keys: Vec<String>,
}

impl User {
    pub async fn login(state: &GlobalState, slug: &str, password: &str) -> Result<Self> {
        let mut user = Self::load(state, slug).await?;

        match verify_password_hash(password, &user.password_hash) {
            PasswordVerification::Invalid | PasswordVerification::Error => {
                Err(anyhow!("Invalid Password"))
            }
            PasswordVerification::ValidNeedsRehash { new_hash } => {
                user.password_hash = new_hash;
                user.last_login = Some(time::OffsetDateTime::now_utc());
                user.save(state).await?;
                Ok(user)
            }
            PasswordVerification::Valid => {
                user.last_login = Some(time::OffsetDateTime::now_utc());
                user.save(state).await?;
                Ok(user)
            }
        }
    }

    // ToDo: should probably double/triple check this implementation
    pub fn validate_ssh_key(&self, ssh_key: &ssh_key::PublicKey) -> Result<()> {
        let Ok(ssh_key) = ssh_key.to_openssh() else {
            return Err(anyhow!("Invalid ssh_key"));
        };
        for key in &self.ssh_keys {
            let Ok(mut key) = ssh_key::PublicKey::from_openssh(key) else {
                continue;
            };
            key.set_comment("");
            if let Ok(key) = key.to_openssh()
                && key == ssh_key
            {
                return Ok(());
            }
        }
        Err(anyhow!("PublicKey doesn't match user"))
    }

    pub async fn load(state: &GlobalState, slug: &str) -> Result<Self> {
        if validate_username(slug).is_err() {
            return Err(anyhow!("Invalid username"));
        }
        let path = state.config.git_root.join(format!("!{slug}.json"));
        let data = tokio::fs::read(path).await?;
        let data = String::from_utf8_lossy(&data);
        let user: User = serde_json::from_str(&data)?;

        Ok(user)
    }

    // ToDo: would be better to do things atomic, should be good enough for now though
    pub async fn save(&self, state: &GlobalState) -> Result<()> {
        if validate_username(&self.slug).is_err() {
            return Err(anyhow!("Invalid username"));
        }
        let path = state.config.git_root.join(format!("!{}.json", self.slug));
        let data = serde_json::to_string(&self)?;
        tokio::fs::write(path, data).await?;

        Ok(())
    }

    pub fn new(name: &str, email: &str, password: &str) -> Result<Self> {
        let slug = slugify(name);
        if validate_username(&slug).is_err() {
            return Err(anyhow!("Invalid username/slug"));
        }

        let password_hash = hash_password(password).map_err(|_| anyhow!("Can't hash password"))?;

        Ok(Self {
            id: Uuid::new_v4(),
            created_at: time::OffsetDateTime::now_utc(),
            last_login: None,
            name: name.to_string(),
            slug,
            email: email.to_string(),
            password_hash,
            description: "".to_string(),
            website: "".to_string(),

            default_main_branch: "main".to_string(),
            ssh_keys: vec![],
        })
    }

    pub async fn projects(&self, state: &GlobalState) -> Result<Vec<Project>> {
        let mut ret = vec![];

        let path = state.config.git_root.join(&self.slug);
        let Ok(mut entries) = tokio::fs::read_dir(path).await else {
            return Ok(ret);
        };

        while let Some(entry) = entries.next_entry().await? {
            let meta = entry.metadata().await?;
            if meta.is_dir() {
                let file_name = entry.file_name();
                let project_slug = file_name.to_string_lossy();
                if let Ok(project) = Project::load(state, &self.slug, &project_slug).await {
                    ret.push(project);
                }
            };
        }
        Ok(ret)
    }

    pub fn uri(&self) -> String {
        format!("/~{}", self.slug)
    }
}

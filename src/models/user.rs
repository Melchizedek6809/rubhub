use anyhow::{Result, anyhow};
use russh::keys::ssh_key;
use serde::{Deserialize, Serialize};

use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    GlobalState, Project,
    services::{
        fs::atomic_write,
        password::{PasswordVerification, hash_password, verify_password_hash},
        user_profile::{
            self, PROFILE_REPO, load_ssh_keys, load_user_profile, profile_repo_exists,
            save_ssh_keys, save_user_profile,
        },
        validation::{slugify, validate_username},
    },
};

/// Credentials stored in !{username}.json
/// This struct supports both the legacy format (with all fields) and the new format (credentials only)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserCredentials {
    pub id: Uuid,
    // No serde attribute - accepts both legacy array format and new RFC3339 format
    pub created_at: OffsetDateTime,
    #[serde(default)]
    pub last_login: Option<OffsetDateTime>,

    pub slug: String,
    pub password_hash: String,

    // Legacy fields - used for backward compatibility during migration
    // These are read from old JSON files but not written to new ones
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub website: Option<String>,
    #[serde(default)]
    pub default_main_branch: Option<String>,
    #[serde(default)]
    pub ssh_keys: Option<Vec<String>>,
}

/// Minimal credentials to write to JSON (no legacy fields)
#[derive(Clone, Debug, Serialize)]
struct UserCredentialsMinimal {
    id: Uuid,
    created_at: OffsetDateTime,
    last_login: Option<OffsetDateTime>,
    slug: String,
    password_hash: String,
}

#[derive(Clone, Debug, PartialEq)]
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

    pub fn validate_ssh_key(&self, ssh_key: &ssh_key::PublicKey) -> Result<()> {
        for key in &self.ssh_keys {
            let Ok(key) = ssh_key::PublicKey::from_openssh(key) else {
                continue;
            };
            if key.key_data() == ssh_key.key_data() {
                return Ok(());
            }
        }
        Err(anyhow!("PublicKey doesn't match user"))
    }

    pub async fn load(state: &GlobalState, slug: &str) -> Result<Self> {
        if validate_username(slug).is_err() {
            return Err(anyhow!("Invalid username"));
        }

        // 1. Load credentials from JSON (required - determines if user exists)
        let path = state.config.git_root.join(format!("!{slug}.json"));
        let data = tokio::fs::read(path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow!("No such user")
            } else {
                anyhow!(e)
            }
        })?;
        let data = String::from_utf8_lossy(&data);
        let creds: UserCredentials = serde_json::from_str(&data)?;

        // 2. Load profile from .profile repo (optional - graceful degradation)
        let (profile_fm, description) = load_user_profile(state, slug).await;

        // 3. Load SSH keys from .profile (optional - graceful degradation)
        let ssh_keys = load_ssh_keys(state, slug).await;

        // 4. Merge data: prefer .profile data, fall back to legacy JSON fields, then defaults
        Ok(Self {
            id: creds.id,
            slug: creds.slug,
            password_hash: creds.password_hash,
            created_at: profile_fm.created_at.unwrap_or(creds.created_at),
            last_login: creds.last_login,
            name: profile_fm
                .name
                .or(creds.name)
                .unwrap_or_else(|| slug.to_string()),
            email: profile_fm.email.or(creds.email).unwrap_or_default(),
            website: profile_fm.website.or(creds.website).unwrap_or_default(),
            description: if description.is_empty() {
                creds.description.unwrap_or_default()
            } else {
                description
            },
            default_main_branch: profile_fm
                .default_main_branch
                .or(creds.default_main_branch)
                .unwrap_or_else(|| "main".to_string()),
            ssh_keys: if ssh_keys.is_empty() {
                creds.ssh_keys.unwrap_or_default()
            } else {
                ssh_keys
            },
        })
    }

    pub async fn save(&self, state: &GlobalState) -> Result<()> {
        if validate_username(&self.slug).is_err() {
            return Err(anyhow!("Invalid username"));
        }

        // 1. Save credentials to JSON (minimal format)
        let creds = UserCredentialsMinimal {
            id: self.id,
            slug: self.slug.clone(),
            password_hash: self.password_hash.clone(),
            created_at: self.created_at,
            last_login: self.last_login,
        };
        let path = state.config.git_root.join(format!("!{}.json", self.slug));
        let data = serde_json::to_string(&creds)?;
        atomic_write(path, data).await?;

        // 2. Ensure .profile repo exists
        if !profile_repo_exists(state, &self.slug).await {
            user_profile::create_profile_repo(
                state,
                &self.slug,
                &self.name,
                &self.email,
                &self.website,
                &self.default_main_branch,
                &self.description,
                self.created_at,
            )
            .await?;
        } else {
            // 3. Save profile to .profile/README.md
            save_user_profile(
                state,
                &self.slug,
                &self.name,
                &self.email,
                &self.website,
                &self.default_main_branch,
                &self.description,
                self.created_at,
            )
            .await?;

            // 4. Save SSH keys to .profile/authorized_keys
            save_ssh_keys(state, &self.slug, &self.ssh_keys, &self.name, &self.email).await?;
        }

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

                // Skip .profile repo - it's a special system repo
                if project_slug == PROFILE_REPO {
                    continue;
                }

                if let Ok(project) = Project::load(state, &self.slug, &project_slug).await {
                    ret.push(project);
                }
            };
        }
        Ok(ret)
    }

    pub async fn sidebar_projects(&self, state: &GlobalState) -> Vec<Project> {
        let mut projects = self.projects(state).await.unwrap_or_default();

        // Sort alphabetically by project name (case-insensitive)
        projects.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        // Limit to 10 projects
        projects.truncate(10);

        projects
    }

    pub fn uri(&self) -> String {
        format!("/~{}", self.slug)
    }
}

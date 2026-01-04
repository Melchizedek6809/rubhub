use anyhow::{Result, anyhow};
use rubhub_auth_store::User;
use russh::keys::ssh_key;

use crate::{
    GlobalState, Project,
};

pub trait UserModel {
    fn validate_ssh_key(&self, ssh_key: &ssh_key::PublicKey) -> Result<()>;
    fn projects(&self, state: &GlobalState) -> impl std::future::Future<Output = Result<Vec<Project>>> + Send;
    fn sidebar_projects(&self, state: &GlobalState) -> impl std::future::Future<Output = Vec<Project>> + Send;
    fn uri(&self) -> String;
}

impl UserModel for User {
    fn validate_ssh_key(&self, ssh_key: &ssh_key::PublicKey) -> Result<()> {
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

    async fn projects(&self, state: &GlobalState) -> Result<Vec<Project>> {
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

    async fn sidebar_projects(&self, state: &GlobalState) -> Vec<Project> {
        let mut projects = self.projects(state).await.unwrap_or_default();

        // Sort alphabetically by project name (case-insensitive)
        projects.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        // Limit to 10 projects
        projects.truncate(10);

        projects
    }

    fn uri(&self) -> String {
        format!("/~{}", self.slug)
    }
}

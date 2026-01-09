use anyhow::Result;
use rubhub_auth_store::User;

use crate::{GlobalState, Project};

pub trait UserModel {
    fn projects(
        &self,
        state: &GlobalState,
    ) -> impl std::future::Future<Output = Result<Vec<Project>>> + Send;
    fn sidebar_projects(
        &self,
        state: &GlobalState,
    ) -> impl std::future::Future<Output = Vec<Project>> + Send;
    fn uri(&self) -> String;
}

impl UserModel for User {
    async fn projects(&self, state: &GlobalState) -> Result<Vec<Project>> {
        let project_infos = state.auth.get_projects_for_owner(&self.slug);

        let projects: Vec<Project> = project_infos
            .iter()
            .filter_map(|info| Project::from_project_info(info))
            .collect();

        Ok(projects)
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

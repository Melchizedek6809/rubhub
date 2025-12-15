use askama::Template;

use crate::{
    AccessType, Project, User,
    services::repository::{GitRefInfo, GitSummary},
    views::ThemedRender,
};

#[derive(Template)]
#[template(path = "project.html")]
struct ProjectTemplate<'a> {
    owner: &'a User,
    project: &'a Project,
    access_level: AccessType,
    ssh_clone_url: String,
    selected_branch: String,
    summary: GitSummary,
    info: Option<GitRefInfo>,
    readme_html: Option<String>,
}

pub async fn project_with_access(
    owner: User,
    project: Project,
    access_level: AccessType,
    ssh_clone_url: String,
    summary: GitSummary,
    info: Option<GitRefInfo>,
    readme_html: Option<String>,
) -> String {
    let selected_branch = info
        .as_ref()
        .map(|i| i.branch_name.to_string())
        .unwrap_or_default();

    ProjectTemplate {
        owner: &owner,
        project: &project,
        access_level,
        ssh_clone_url,
        summary,
        info,
        selected_branch,
        readme_html,
    }
    .render_with_theme()
}

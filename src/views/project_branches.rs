use askama::Template;

use crate::{
    AccessType, Project, User,
    services::repository::GitRefInfo,
    views::{extract_html_parts, theme_render},
};

#[derive(Template)]
#[template(path = "project_branches.html")]
struct ProjectBranchesTemplate<'a> {
    owner: &'a User,
    project: &'a Project,
    access_level: AccessType,
    branches: Vec<GitRefInfo>,
}

pub async fn project_branches(
    owner: User,
    project: Project,
    access_level: AccessType,
    branches: Vec<GitRefInfo>,
) -> String {
    let contents = ProjectBranchesTemplate {
        owner: &owner,
        project: &project,
        access_level,
        branches,
    }
    .render()
    .unwrap();

    let parts = extract_html_parts(&contents);

    theme_render(parts.0, parts.1).await
}

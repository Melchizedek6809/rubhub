use askama::Template;

use crate::{AccessType, Project, User, services::repository::GitRefInfo, views::ThemedRender};

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
    ProjectBranchesTemplate {
        owner: &owner,
        project: &project,
        access_level,
        branches,
    }
    .render_with_theme()
}

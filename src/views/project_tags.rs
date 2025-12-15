use askama::Template;

use crate::{AccessType, Project, User, services::repository::GitRefInfo, views::ThemedRender};

#[derive(Template)]
#[template(path = "project_tags.html")]
struct ProjectTagsTemplate<'a> {
    owner: &'a User,
    project: &'a Project,
    access_level: AccessType,
    tags: Vec<GitRefInfo>,
}

pub async fn project_tags(
    owner: User,
    project: Project,
    access_level: AccessType,
    tags: Vec<GitRefInfo>,
) -> String {
    ProjectTagsTemplate {
        owner: &owner,
        project: &project,
        access_level,
        tags,
    }
    .render_with_theme()
}

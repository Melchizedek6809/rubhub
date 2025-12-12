use askama::Template;

use crate::{
    AccessType, Project, User,
    services::repository::GitRefInfo,
    views::{extract_html_parts, theme_render},
};

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
    let contents = ProjectTagsTemplate {
        owner: &owner,
        project: &project,
        access_level,
        tags,
    }
    .render()
    .unwrap();

    let parts = extract_html_parts(&contents);

    theme_render(parts.0, parts.1).await
}

use askama::Template;

use crate::{
    AccessType, Project, User,
    views::{extract_html_parts, theme_render},
};

#[derive(Template)]
#[template(path = "project_settings.html")]
struct ProjectSettingsTemplate<'a> {
    owner: &'a User,
    project: &'a Project,
    message: Option<&'a str>,
}

pub async fn project_settings(owner: User, project: Project, message: Option<&str>) -> String {
    let contents = ProjectSettingsTemplate {
        owner: &owner,
        project: &project,
        message,
    }
    .render()
    .unwrap();

    let parts = extract_html_parts(&contents);

    theme_render(parts.0, parts.1).await
}

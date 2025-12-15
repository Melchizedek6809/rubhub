use askama::Template;

use crate::{AccessType, Project, User, views::ThemedRender};

#[derive(Template)]
#[template(path = "project_settings.html")]
struct ProjectSettingsTemplate<'a> {
    owner: &'a User,
    project: &'a Project,
    message: Option<&'a str>,
}

pub async fn project_settings(owner: User, project: Project, message: Option<&str>) -> String {
    ProjectSettingsTemplate {
        owner: &owner,
        project: &project,
        message,
    }
    .render_with_theme()
}

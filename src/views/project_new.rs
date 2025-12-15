use askama::Template;

use crate::{AccessType, views::ThemedRender};

#[derive(Template)]
#[template(path = "project_new.html")]
struct NewProjectTemplate<'a> {
    message: Option<&'a str>,
    public_access: &'a str,
}

pub async fn new_project(message: Option<&str>, public_access: AccessType) -> String {
    NewProjectTemplate {
        message,
        public_access: public_access.as_str(),
    }
    .render_with_theme()
}

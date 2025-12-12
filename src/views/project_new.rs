use askama::Template;

use crate::{
    AccessType,
    views::{extract_html_parts, theme_render},
};

#[derive(Template)]
#[template(path = "project_new.html")]
struct NewProjectTemplate<'a> {
    message: Option<&'a str>,
    public_access: &'a str,
}

pub async fn new_project(message: Option<&str>, public_access: AccessType) -> String {
    let contents = NewProjectTemplate {
        message,
        public_access: public_access.as_str(),
    }
    .render()
    .unwrap();

    let parts = extract_html_parts(&contents);

    theme_render(parts.0, parts.1).await
}

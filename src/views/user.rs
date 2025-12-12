use askama::Template;

use crate::{
    ProjectSummary, User,
    views::{extract_html_parts, theme_render},
};

#[derive(Template)]
#[template(path = "user.html")]
struct UserTemplate<'a> {
    user: &'a User,
    projects: &'a [ProjectSummary<'a>],
    is_owner: bool,
}

pub async fn profile(user: &User, projects: &[ProjectSummary<'_>], is_owner: bool) -> String {
    let contents = UserTemplate {
        user,
        projects,
        is_owner,
    }
    .render()
    .unwrap();

    let parts = extract_html_parts(&contents);

    theme_render(parts.0, parts.1).await
}

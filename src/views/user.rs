use askama::Template;

use crate::{ProjectSummary, User, views::ThemedRender};

#[derive(Template)]
#[template(path = "user.html")]
struct UserTemplate<'a> {
    user: &'a User,
    projects: &'a [ProjectSummary<'a>],
    is_owner: bool,
}

pub async fn profile(user: &User, projects: &[ProjectSummary<'_>], is_owner: bool) -> String {
    UserTemplate {
        user,
        projects,
        is_owner,
    }
    .render_with_theme()
}

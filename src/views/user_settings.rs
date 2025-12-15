use askama::Template;

use crate::{User, views::ThemedRender};

#[derive(Template)]
#[template(path = "user_settings.html")]
struct UserSettingsTemplate<'a> {
    user: &'a User,
    ssh_keys: &'a [String],
    message: Option<&'a str>,
}

pub async fn settings(user: User, ssh_keys: &[String], message: Option<&str>) -> String {
    UserSettingsTemplate {
        user: &user,
        ssh_keys,
        message,
    }
    .render_with_theme()
}

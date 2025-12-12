use askama::Template;

use crate::{
    User,
    views::{extract_html_parts, theme_render},
};

#[derive(Template)]
#[template(path = "user_settings.html")]
struct UserSettingsTemplate<'a> {
    user: &'a User,
    ssh_keys: &'a [String],
    message: Option<&'a str>,
}

pub async fn settings(user: User, ssh_keys: &[String], message: Option<&str>) -> String {
    let contents = UserSettingsTemplate {
        user: &user,
        ssh_keys,
        message,
    }
    .render()
    .unwrap();

    let parts = extract_html_parts(&contents);

    theme_render(parts.0, parts.1).await
}

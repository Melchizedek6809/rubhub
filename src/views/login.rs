use askama::Template;

use crate::views::{extract_html_parts, theme_render};

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate<'a> {
    message: Option<&'a str>,
}

pub async fn login(message: Option<&str>) -> String {
    let contents = LoginTemplate { message }.render().unwrap();

    let parts = extract_html_parts(&contents);

    theme_render(parts.0, parts.1).await
}

use askama::Template;

use crate::views::{extract_html_parts, theme_render};

#[derive(Template)]
#[template(path = "404.html")]
struct NotFoundTemplate;

pub async fn not_found() -> String {
    let contents = NotFoundTemplate.render().unwrap();
    let (head, body) = extract_html_parts(&contents);
    theme_render(head, body).await
}

use askama::Template;

use crate::views::{extract_html_parts, theme_render};

#[derive(Template)]
#[template(path = "contact.html")]
struct ContactTemplate;

pub async fn contact() -> String {
    let contents = ContactTemplate.render().unwrap();
    let (head, body) = extract_html_parts(&contents);
    theme_render(head, body).await
}

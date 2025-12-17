use askama::Template;
use axum::response::Html;

use crate::views::ThemedRender;

#[derive(Template)]
#[template(path = "contact.html")]
struct ContactTemplate;

pub async fn contact() -> Html<String> {
    Html(ContactTemplate.render_with_theme())
}

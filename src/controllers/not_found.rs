use askama::Template;
use axum::{http::StatusCode, response::Html};

use crate::views::ThemedRender;

#[derive(Template)]
#[template(path = "404.html")]
struct NotFoundTemplate;

pub async fn not_found() -> (StatusCode, Html<String>) {
    (
        StatusCode::NOT_FOUND,
        Html(NotFoundTemplate.render_with_theme()),
    )
}

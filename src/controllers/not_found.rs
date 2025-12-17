use askama::Template;
use axum::{http::StatusCode, response::Html};

use crate::{User, views::ThemedRender};

#[derive(Template)]
#[template(path = "404.html")]
struct NotFoundTemplate<'a> {
    logged_in_user: Option<&'a User>,
}

pub async fn not_found() -> (StatusCode, Html<String>) {
    let template = NotFoundTemplate {
        logged_in_user: None,
    };
    (
        StatusCode::NOT_FOUND,
        Html(template.render_with_theme()),
    )
}

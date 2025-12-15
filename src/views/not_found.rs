use askama::Template;

use crate::views::ThemedRender;

#[derive(Template)]
#[template(path = "404.html")]
struct NotFoundTemplate;

pub async fn not_found() -> String {
    NotFoundTemplate.render_with_theme()
}

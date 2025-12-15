use askama::Template;

use crate::views::ThemedRender;

#[derive(Template)]
#[template(path = "contact.html")]
struct ContactTemplate;

pub async fn contact() -> String {
    ContactTemplate.render_with_theme()
}

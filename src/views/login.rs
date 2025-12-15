use askama::Template;

use crate::views::ThemedRender;

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate<'a> {
    message: Option<&'a str>,
}

pub async fn login(message: Option<&str>) -> String {
    LoginTemplate { message }.render_with_theme()
}

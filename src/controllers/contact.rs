use askama::Template;
use axum::{extract::State, response::Html};
use tower_cookies::Cookies;

use crate::{GlobalState, User, views::ThemedRender, services::session};

#[derive(Template)]
#[template(path = "contact.html")]
struct ContactTemplate<'a> {
    logged_in_user: Option<&'a User>,
}

pub async fn contact(State(state): State<GlobalState>, cookies: Cookies) -> Html<String> {
    let logged_in_user = session::current_user(&state, &cookies).await.ok();
    let template = ContactTemplate {
        logged_in_user: logged_in_user.as_ref(),
    };
    Html(template.render_with_theme())
}

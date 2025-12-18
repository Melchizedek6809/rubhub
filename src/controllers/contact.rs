use askama::Template;
use axum::{extract::State, response::Html};
use tower_cookies::Cookies;

use crate::{GlobalState, Project, User, services::session, views::ThemedRender};

#[derive(Template)]
#[template(path = "contact.html")]
struct ContactTemplate<'a> {
    logged_in_user: Option<&'a User>,
    sidebar_projects: Vec<Project>,
}

pub async fn contact(State(state): State<GlobalState>, cookies: Cookies) -> Html<String> {
    let logged_in_user = session::current_user(&state, &cookies).await.ok();

    let sidebar_projects = if let Some(ref user) = logged_in_user {
        user.sidebar_projects(&state).await
    } else {
        vec![]
    };

    let template = ContactTemplate {
        logged_in_user: logged_in_user.as_ref(),
        sidebar_projects,
    };
    Html(template.render_with_theme())
}

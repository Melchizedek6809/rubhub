use askama::Template;
use axum::{
    body::Body,
    extract::State,
    http::{Response, StatusCode},
    response::{Html, IntoResponse},
};
use tower_cookies::Cookies;

use crate::{GlobalState, Project, User, services::session, views::ThemedRender};

#[derive(Template)]
#[template(path = "404.html")]
struct NotFoundTemplate<'a> {
    logged_in_user: Option<&'a User>,
    sidebar_projects: Vec<Project>,
}

pub fn not_found(logged_in_user: Option<User>, sidebar_projects: Vec<Project>) -> Response<Body> {
    let template = NotFoundTemplate {
        logged_in_user: logged_in_user.as_ref(),
        sidebar_projects,
    };
    (StatusCode::NOT_FOUND, Html(template.render_with_theme())).into_response()
}

pub async fn not_found_get(State(state): State<GlobalState>, cookies: Cookies) -> Response<Body> {
    let logged_in_user = session::current_user(&state, &cookies).await.ok();

    let sidebar_projects = if let Some(ref user) = logged_in_user {
        user.sidebar_projects(&state).await
    } else {
        vec![]
    };

    not_found(logged_in_user, sidebar_projects)
}

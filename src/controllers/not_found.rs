use askama::Template;
use axum::{
    body::Body,
    extract::State,
    http::{Response, StatusCode},
    response::{Html, IntoResponse},
};
use tower_cookies::Cookies;

use crate::{models::ContentPage, services::session, views::ThemedRender, GlobalState, Project, User};

#[derive(Template)]
#[template(path = "404.html")]
struct NotFoundTemplate<'a> {
    logged_in_user: Option<&'a User>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
}

pub fn not_found(
    logged_in_user: Option<User>,
    content_pages: Vec<ContentPage>,
) -> Response<Body> {
    let template = NotFoundTemplate {
        logged_in_user: logged_in_user.as_ref(),
        sidebar_projects: vec![],
        content_pages,
    };
    (StatusCode::NOT_FOUND, Html(template.render_with_theme())).into_response()
}

pub async fn not_found_get(State(state): State<GlobalState>, cookies: Cookies) -> Response<Body> {
    let logged_in_user = session::current_user(&state, &cookies).await.ok();

    not_found(logged_in_user, state.config.content_pages.clone())
}

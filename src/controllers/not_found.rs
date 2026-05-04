use std::sync::Arc;

use askama::Template;
use axum::{
    body::Body,
    extract::State,
    http::{Response, StatusCode},
    response::{Html, IntoResponse},
};
use tower_cookies::Cookies;

use crate::{
    GlobalState, Project, User, controllers::context::PageContext, models::ContentPage,
    services::meta::PageMeta, views::ThemedRender,
};

#[derive(Template)]
#[template(path = "404.html")]
struct NotFoundTemplate {
    logged_in_user: Option<Arc<User>>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    meta: PageMeta,
}

pub fn not_found(
    logged_in_user: Option<Arc<User>>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
) -> Response<Body> {
    let template = NotFoundTemplate {
        logged_in_user,
        sidebar_projects,
        content_pages,
        meta: PageMeta::new(
            "404 - Page not found",
            "This RubHub page could not be found.",
        )
        .robots("noindex,follow"),
    };
    (StatusCode::NOT_FOUND, Html(template.render_with_theme())).into_response()
}

pub async fn not_found_get(State(state): State<GlobalState>, cookies: Cookies) -> Response<Body> {
    PageContext::load(&state, &cookies).await.not_found()
}

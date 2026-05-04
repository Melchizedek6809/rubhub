use std::sync::Arc;

use askama::Template;
use axum::{
    body::Body,
    extract::State,
    http::{StatusCode, header},
    response::Response,
};
use tower_cookies::Cookies;

use crate::{
    GlobalState, Project, User,
    controllers::context::PageContext,
    models::ContentPage,
    services::{content, meta::PageMeta},
    views::ThemedRender,
};

#[derive(Template)]
#[template(path = "content_page.html")]
struct ContentPageTemplate<'a> {
    page_title: &'a str,
    content_html: String,
    logged_in_user: Option<Arc<User>>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    meta: PageMeta,
}

pub async fn render_content_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
    page: ContentPage,
) -> Response<Body> {
    let page_context = PageContext::load(&state, &cookies).await;

    // Fetch and render markdown
    let content_html = match content::render_content(&page, &state).await {
        Ok(html) => html,
        Err(e) => {
            eprintln!(
                "Error rendering content page '{}' ({}:{}): {}",
                page.title, page.repo_owner, page.repo_slug, e
            );
            return page_context.not_found();
        }
    };

    let template = ContentPageTemplate {
        page_title: &page.title,
        content_html,
        logged_in_user: page_context.logged_in_user,
        sidebar_projects: page_context.sidebar_projects,
        content_pages: page_context.content_pages,
        meta: PageMeta::new(
            format!("{} - RubHub", page.title),
            format!("{} on RubHub.", page.title),
        )
        .canonical(&state.config.base_url, &page.url_path()),
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from(template.render_with_theme()))
        .unwrap()
}

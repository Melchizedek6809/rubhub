use askama::Template;
use axum::{body::Body, extract::State, http::StatusCode, response::Response};
use tower_cookies::Cookies;

use crate::{
    models::ContentPage, services::session, views::ThemedRender, GlobalState, Project, User,
};

#[derive(Template)]
#[template(path = "content_page.html")]
struct ContentPageTemplate<'a> {
    page_title: &'a str,
    content_html: String,
    logged_in_user: Option<&'a User>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
}

pub async fn render_content_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
    page: ContentPage,
) -> Response<Body> {
    let logged_in_user = session::current_user(&state, &cookies).await.ok();

    let sidebar_projects = if let Some(ref user) = logged_in_user {
        user.sidebar_projects(&state).await
    } else {
        vec![]
    };

    // Fetch and render markdown
    let content_html = match page.render_content(&state).await {
        Ok(html) => html,
        Err(e) => {
            eprintln!(
                "Error rendering content page '{}' ({}:{}): {}",
                page.title, page.repo_owner, page.repo_slug, e
            );
            return crate::controllers::not_found(
                logged_in_user,
                state.config.content_pages.clone(),
            );
        }
    };

    let template = ContentPageTemplate {
        page_title: &page.title,
        content_html,
        logged_in_user: logged_in_user.as_ref(),
        sidebar_projects,
        content_pages: state.config.content_pages.clone(),
    };

    Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(template.render_with_theme()))
        .unwrap()
}

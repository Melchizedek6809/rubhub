use std::sync::Arc;

use crate::{
    AccessType, GlobalState, Project, ProjectSummary, User,
    controllers::context::PageContext,
    models::ContentPage,
    services::{content, meta::PageMeta},
    views::ThemedRender,
};
use askama::Template;
use axum::{extract::State, response::Html};
use tower_cookies::Cookies;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    featured: &'a [ProjectSummary<'a>],
    logged_in_user: Option<Arc<User>>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    index_content: String,
    meta: PageMeta,
}

pub async fn index(State(state): State<GlobalState>, cookies: Cookies) -> Html<String> {
    let page_context = PageContext::load(&state, &cookies).await;

    let mut projects: Vec<(Arc<User>, Project)> = vec![];
    for project_path in &state.config.featured_projects {
        if let Ok((owner, project)) = Project::load_by_path(&state, project_path.clone()).await
            && project.public_access != AccessType::None
        {
            projects.push((owner, project));
        }
    }

    let featured: Vec<ProjectSummary<'_>> = projects
        .iter()
        .map(|(owner, project)| ProjectSummary {
            name: project.name.as_str(),
            slug: project.slug.as_str(),
            owner_slug: owner.slug.as_str(),
            owner_name: owner.name.as_str(),
            description: project.description.as_str(),
        })
        .collect();

    let index_content = match &state.config.index_content {
        Some(page) => {
            content::render_content(page, &state).await.unwrap_or("Error rendering INDEX_CONTENT!".to_string())
        },
        None => "<p>Welcome to your new <a href=\"https://rubhub.net/~ben/rubhub\">rubhub</a> instance, please specify INDEX_CONTENT to remove this message.</p>".to_string(),
    };

    let template = IndexTemplate {
        featured: &featured,
        logged_in_user: page_context.logged_in_user,
        sidebar_projects: page_context.sidebar_projects,
        content_pages: page_context.content_pages,
        index_content,
        meta: PageMeta::new(
            "RubHub",
            "RubHub is a lightweight git forge built around plain git repositories.",
        )
        .canonical(&state.config.base_url, "/"),
    };

    Html(template.render_with_theme())
}

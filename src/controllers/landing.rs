use std::sync::Arc;

use crate::{
    GlobalState, Project, ProjectSummary, User, UserModel, models::ContentPage, services::{content, session},
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
}

pub async fn index(State(state): State<GlobalState>, cookies: Cookies) -> Html<String> {
    let logged_in_user = session::current_user(&state, &cookies).await.ok();

    let sidebar_projects = if let Some(ref user) = logged_in_user {
        user.sidebar_projects(&state).await
    } else {
        vec![]
    };

    let mut projects: Vec<(Arc<User>, Project)> = vec![];
    for project_path in &state.config.featured_projects {
        if let Ok(tuple) = Project::load_by_path(&state, project_path.clone()).await {
            projects.push(tuple);
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
        logged_in_user,
        sidebar_projects,
        content_pages: state.config.content_pages.clone(),
        index_content,
    };

    Html(template.render_with_theme())
}

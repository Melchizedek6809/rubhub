use crate::{
    models::ContentPage, services::session, views::ThemedRender, GlobalState, Project,
    ProjectSummary, User,
};
use askama::Template;
use axum::{extract::State, response::Html};
use tower_cookies::Cookies;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    featured: &'a [ProjectSummary<'a>],
    logged_in_user: Option<&'a User>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
}

pub async fn index(State(state): State<GlobalState>, cookies: Cookies) -> Html<String> {
    let logged_in_user = session::current_user(&state, &cookies).await.ok();

    let sidebar_projects = if let Some(ref user) = logged_in_user {
        user.sidebar_projects(&state).await
    } else {
        vec![]
    };

    let mut projects: Vec<(User, Project)> = vec![];
    if let Ok(tuple) = Project::load_by_path(&state, "ben/rubhub".to_string()).await {
        projects.push(tuple);
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

    let template = IndexTemplate {
        featured: &featured,
        logged_in_user: logged_in_user.as_ref(),
        sidebar_projects,
        content_pages: state.config.content_pages.clone(),
    };

    Html(template.render_with_theme())
}

use std::sync::Arc;

use askama::Template;
use axum::{body::Body, extract::State, http::Response, response::Html};
use tower_cookies::Cookies;

use crate::{
    GlobalState, Project, ProjectSummary, User, UserModel, models::ContentPage, services::session,
    views::ThemedRender,
};

#[derive(Template)]
#[template(path = "project/list.html")]
struct ProjectsListTemplate<'a> {
    projects: &'a [ProjectSummary<'a>],
    logged_in_user: Option<Arc<User>>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
}

pub async fn all_projects_list(
    State(state): State<GlobalState>,
    cookies: Cookies,
) -> Result<Html<String>, Response<Body>> {
    let logged_in_user = session::current_user(&state, &cookies).await.ok();

    let sidebar_projects = if let Some(ref user) = logged_in_user {
        user.sidebar_projects(&state).await
    } else {
        vec![]
    };

    // Get all public projects from auth_store
    let public_infos = state.auth.get_public_projects();

    let mut all_projects: Vec<(Arc<User>, Project)> = public_infos
        .iter()
        .filter_map(|info| {
            let project = Project::from_project_info(info)?;
            let user = state.auth.get_user(&project.owner)?;
            Some((user, project))
        })
        .collect();

    // Sort alphabetically by project name
    all_projects.sort_by(|a, b| a.1.name.to_lowercase().cmp(&b.1.name.to_lowercase()));

    // Convert to ProjectSummary
    let summaries: Vec<_> = all_projects
        .iter()
        .map(|(user, project)| ProjectSummary {
            name: project.name.as_str(),
            slug: project.slug.as_str(),
            owner_name: user.name.as_str(),
            owner_slug: user.slug.as_str(),
            description: project.description.as_str(),
        })
        .collect();

    let template = ProjectsListTemplate {
        projects: &summaries,
        logged_in_user,
        sidebar_projects,
        content_pages: state.config.content_pages.clone(),
    };
    Ok(Html(template.render_with_theme()))
}

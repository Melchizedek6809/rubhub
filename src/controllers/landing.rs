use crate::{GlobalState, Project, ProjectSummary, User, views};
use axum::{extract::State, response::Html};

pub async fn index(State(state): State<GlobalState>) -> Html<String> {
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

    Html(views::index::index(&featured).await)
}

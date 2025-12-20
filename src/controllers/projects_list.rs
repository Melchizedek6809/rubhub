use askama::Template;
use axum::{body::Body, extract::State, http::Response, response::Html};
use tower_cookies::Cookies;

use crate::{
    GlobalState, Project, ProjectSummary, User,
    models::{AccessType, ContentPage},
    services::session,
    views::ThemedRender,
};

#[derive(Template)]
#[template(path = "projects_list.html")]
struct ProjectsListTemplate<'a> {
    projects: &'a [ProjectSummary<'a>],
    logged_in_user: Option<&'a User>,
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

    // Get all users
    let mut all_projects = Vec::new();
    let Ok(mut entries) = tokio::fs::read_dir(&state.config.git_root).await else {
        // If we can't read the git_root, just return empty list
        let template = ProjectsListTemplate {
            projects: &[],
            logged_in_user: logged_in_user.as_ref(),
            sidebar_projects,
            content_pages: state.config.content_pages.clone(),
        };
        return Ok(Html(template.render_with_theme()));
    };

    while let Some(entry) = entries.next_entry().await.ok().flatten() {
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        // User metadata files are named !{username}.json
        if file_name_str.starts_with('!') && file_name_str.ends_with(".json") {
            let username = file_name_str
                .strip_prefix('!')
                .and_then(|s| s.strip_suffix(".json"))
                .unwrap();

            // Load the user
            if let Ok(user) = User::load(&state, username).await {
                // Get their projects
                if let Ok(projects) = user.projects(&state).await {
                    for project in projects {
                        // Only include projects that allow at least Read access
                        if project.public_access.is_allowed(AccessType::Read) {
                            all_projects.push((user.clone(), project));
                        }
                    }
                }
            }
        }
    }

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
        logged_in_user: logged_in_user.as_ref(),
        sidebar_projects,
        content_pages: state.config.content_pages.clone(),
    };
    Ok(Html(template.render_with_theme()))
}

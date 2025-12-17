use askama::Template;
use axum::{
    body::Body,
    extract::State,
    http::Response,
    response::{Html, IntoResponse},
};
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project, User,
    controllers::not_found,
    extractors::{PathUserProject, PathUserProjectBranch},
    services::{
        repository::{GitRefInfo, GitSummary, get_git_file, get_git_info, get_git_summary},
        session,
    },
    views::ThemedRender,
};

#[derive(Template)]
#[template(path = "project.html")]
struct ProjectTemplate<'a> {
    owner: &'a User,
    project: &'a Project,
    access_level: AccessType,
    ssh_clone_url: String,
    selected_branch: String,
    summary: GitSummary,
    info: Option<GitRefInfo>,
    readme_html: Option<String>,
}

async fn render_project_page(
    state: &GlobalState,
    cookies: Cookies,
    owner: User,
    project: Project,
    branch: Option<String>,
) -> Response<Body> {
    let session_user = session::current_user(state, &cookies).await.ok();
    let access_level = project
        .access_level(session_user.as_ref().map(|user| user.slug.clone()))
        .await;

    if access_level == AccessType::None {
        return not_found().await.into_response();
    }

    let Some(summary) = get_git_summary(state, &owner.slug, &project.slug).await else {
        return not_found().await.into_response();
    };

    let git_user = session_user.map(|u| u.slug).unwrap_or("anon".to_string());

    let ssh_clone_url = format!(
        "ssh://{}@{}/{}/{}",
        git_user, state.config.ssh_public_host, owner.slug, project.slug
    );

    let current = match branch {
        Some(branch) => branch,
        None => project.main_branch.clone(),
    };
    let info = get_git_info(state, &owner.slug, &project.slug, &current, 1, 0).await;

    let readme_html = get_git_file(state, &owner.slug, &project.slug, &current, "README.md").await;
    let readme_html = readme_html
        .map(|b| {
            let str = String::from_utf8_lossy(&b.data);
            let html =
                markdown::to_html_with_options(&str, &markdown::Options::gfm()).unwrap_or_default();

            ammonia::clean(&html)
        })
        .ok();

    let selected_branch = info
        .as_ref()
        .map(|i| i.branch_name.to_string())
        .unwrap_or_default();

    // let tree = get_git_tree(state, &owner.slug, &project.slug, &current, "").await;
    // let tree = tree.unwrap_or_default();

    let template = ProjectTemplate {
        owner: &owner,
        project: &project,
        access_level,
        ssh_clone_url,
        summary,
        info,
        selected_branch,
        readme_html,
    };
    Html(template.render_with_theme()).into_response()
}

pub async fn project_overview_tree_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProjectBranch(owner, project, branch): PathUserProjectBranch,
) -> Response<Body> {
    render_project_page(&state, cookies, owner, project, Some(branch)).await
}

pub async fn project_overview_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, project): PathUserProject,
) -> Response<Body> {
    render_project_page(&state, cookies, owner, project, None).await
}

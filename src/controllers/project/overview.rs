use askama::Template;
use axum::{body::Body, extract::State, http::Response};
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project, User,
    controllers::not_found,
    extractors::PathUserProject,
    models::ContentPage,
    services::{
        markdown::{self, Frontmatter},
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
    http_clone_url: String,
    selected_branch: String,
    summary: GitSummary,
    info: Option<GitRefInfo>,
    readme_html: Option<String>,
    readme_frontmatter: Frontmatter,
    logged_in_user: Option<&'a User>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    active_tab: &'static str,
}

async fn render_project_page(
    state: &GlobalState,
    cookies: Cookies,
    owner: User,
    project: Project,
    branch: Option<String>,
) -> Response<Body> {
    let logged_in_user = session::current_user(state, &cookies).await.ok();

    let sidebar_projects = if let Some(ref user) = logged_in_user {
        user.sidebar_projects(state).await
    } else {
        vec![]
    };

    let access_level = project
        .access_level(logged_in_user.as_ref().map(|user| user.slug.clone()))
        .await;

    if access_level == AccessType::None {
        return not_found(logged_in_user, vec![]);
    }

    let Some(summary) = get_git_summary(state, &owner.slug, &project.slug).await else {
        return not_found(logged_in_user, vec![]);
    };

    let git_user = logged_in_user
        .as_ref()
        .map(|u| u.slug.clone())
        .unwrap_or("anon".to_string());

    let ssh_clone_url = project.ssh_clone_url(&state.config.ssh_public_host, &git_user);
    let http_clone_url = project.http_clone_url(&state.config.base_url);

    let current = match branch {
        Some(branch) => branch,
        None => project.main_branch.clone(),
    };
    let info = get_git_info(state, &owner.slug, &project.slug, &current, 1, 0).await;

    let readme_result =
        get_git_file(state, &owner.slug, &project.slug, &current, "README.md").await;
    let (readme_html, readme_frontmatter) = readme_result
        .map(|b| {
            let content = String::from_utf8_lossy(&b.data);
            let (frontmatter, html) = markdown::parse_and_render(&content);
            (Some(html), frontmatter)
        })
        .unwrap_or((None, vec![]));

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
        http_clone_url,
        summary,
        info,
        selected_branch,
        readme_html,
        readme_frontmatter,
        logged_in_user: logged_in_user.as_ref(),
        sidebar_projects,
        content_pages: state.config.content_pages.clone(),
        active_tab: "overview",
    };
    template.response()
}

pub async fn project_overview_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, project): PathUserProject,
) -> Response<Body> {
    render_project_page(&state, cookies, owner, project, None).await
}

use std::sync::Arc;

use askama::Template;
use axum::{body::Body, extract::State, http::Response};
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project, User, UserModel,
    controllers::context::ProjectRepoContext,
    extractors::PathUserProject,
    models::ContentPage,
    services::{
        markdown::{self, Frontmatter},
        meta::PageMeta,
        repository::{GitRefInfo, GitSummary, get_git_file, get_git_info},
    },
    views::ThemedRender,
};

#[derive(Template)]
#[template(path = "project/overview.html")]
struct ProjectTemplate<'a> {
    owner: Arc<User>,
    project: &'a Project,
    access_level: AccessType,
    ssh_clone_url: String,
    http_clone_url: String,
    selected_branch: String,
    summary: GitSummary,
    info: Option<GitRefInfo>,
    readme_html: Option<String>,
    readme_frontmatter: Frontmatter,
    logged_in_user: Option<Arc<User>>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    active_tab: &'static str,
    meta: PageMeta,
}

async fn render_project_page(
    state: &GlobalState,
    cookies: Cookies,
    owner: Arc<User>,
    project: Project,
    branch: Option<String>,
) -> Response<Body> {
    let repo_context = match ProjectRepoContext::load(state, &cookies, &owner.slug, &project).await
    {
        Ok(context) => context,
        Err(response) => return response,
    };

    let current = match branch {
        Some(branch) => branch,
        None => project.main_branch.clone(),
    };
    let info = get_git_info(state, &owner.slug, &project.slug, &current, 1, 0).await;

    let readme_result =
        get_git_file(state, &owner.slug, &project.slug, &current, "README.md").await;
    let (readme_html, readme_frontmatter) = readme_result
        .map(|b| {
            let content = String::from_utf8_lossy(&b);
            let (frontmatter, html) =
                markdown::MarkdownRenderContext::new(project.uri_blob(&current, ""))
                    .parse_and_render(&content);
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
        meta: PageMeta::new(
            format!("{}/{} - RubHub", owner.name, project.name),
            if project.description.trim().is_empty() {
                format!("{} by {} on RubHub.", project.name, owner.name)
            } else {
                project.description.clone()
            },
        )
        .canonical(&state.config.base_url, &project.uri()),
        owner,
        project: &project,
        access_level: repo_context.project.access_level,
        ssh_clone_url: repo_context.ssh_clone_url,
        http_clone_url: repo_context.http_clone_url,
        summary: repo_context.summary,
        info,
        selected_branch,
        readme_html,
        readme_frontmatter,
        logged_in_user: repo_context.project.page.logged_in_user,
        sidebar_projects: repo_context.project.page.sidebar_projects,
        content_pages: repo_context.project.page.content_pages,
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

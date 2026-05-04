use std::sync::Arc;

use askama::Template;
use axum::{body::Body, extract::State, http::Response};
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project, User, UserModel,
    controllers::context::ProjectRepoContext,
    extractors::{PathUserProjectRef, PathUserProjectRefPath},
    models::ContentPage,
    services::{
        markdown::{self, Frontmatter},
        repository::{
            EntryKind, GitRefInfo, GitSummary, GitTreeEntry, get_git_file, get_git_info,
            get_git_tree,
        },
    },
    views::ThemedRender,
};

#[derive(Template)]
#[template(path = "project/tree.html")]
struct ProjectTreeTemplate<'a> {
    owner: Arc<User>,
    project: &'a Project,
    access_level: AccessType,
    ssh_clone_url: String,
    http_clone_url: String,
    selected_branch: String,
    summary: GitSummary,
    info: Option<GitRefInfo>,
    tree_entries: Vec<GitTreeEntry>,
    current_path: String,
    path_parts: Vec<String>,
    readme_html: Option<String>,
    readme_frontmatter: Frontmatter,
    parent_path: Option<String>,
    logged_in_user: Option<Arc<User>>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    active_tab: &'static str,
}

/// Helper to calculate parent path
fn get_parent_path(path: &str) -> Option<String> {
    if path.is_empty() {
        return None;
    }

    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() <= 1 {
        return Some(String::new()); // Return to root
    }

    Some(parts[..parts.len() - 1].join("/"))
}

pub async fn project_tree_root_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProjectRef(owner, project, git_ref): PathUserProjectRef,
) -> Response<Body> {
    render_tree_page(&state, cookies, owner, project, git_ref, String::new()).await
}

pub async fn project_tree_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProjectRefPath(owner, project, git_ref, path): PathUserProjectRefPath,
) -> Response<Body> {
    render_tree_page(&state, cookies, owner, project, git_ref, path).await
}

async fn render_tree_page(
    state: &GlobalState,
    cookies: Cookies,
    owner: Arc<User>,
    project: Project,
    git_ref: String,
    path: String,
) -> Response<Body> {
    let repo_context = match ProjectRepoContext::load(state, &cookies, &owner.slug, &project).await
    {
        Ok(context) => context,
        Err(response) => return response,
    };

    // Get tree entries
    let tree_result = get_git_tree(state, &owner.slug, &project.slug, &git_ref, &path).await;
    let tree_entries = match tree_result {
        Ok(entries) => entries,
        Err(_) => return repo_context.project.not_found(),
    };

    let info = get_git_info(state, &owner.slug, &project.slug, &git_ref, 1, 0).await;

    let selected_branch = info
        .as_ref()
        .map(|i| i.branch_name.to_string())
        .unwrap_or_else(|| git_ref.clone());

    // Try to load README.md from current directory
    let readme_path = if path.is_empty() {
        "README.md".to_string()
    } else {
        format!("{}/README.md", path)
    };

    let readme_result =
        get_git_file(state, &owner.slug, &project.slug, &git_ref, &readme_path).await;
    let (readme_html, readme_frontmatter) = readme_result
        .ok()
        .filter(|_| {
            // Check if README exists in current directory tree
            tree_entries
                .iter()
                .any(|e| e.filename == "README.md" && e.kind.is_blob())
        })
        .map(|b| {
            let content = String::from_utf8_lossy(&b);
            let base_url = if path.is_empty() {
                project.uri_blob(&git_ref, "")
            } else {
                format!("{}/", project.uri_blob(&git_ref, &path))
            };
            let (frontmatter, html) =
                markdown::MarkdownRenderContext::new(base_url).parse_and_render(&content);
            (Some(html), frontmatter)
        })
        .unwrap_or((None, vec![]));

    let parent_path = get_parent_path(&path);
    let path_parts: Vec<String> = if path.is_empty() {
        vec![]
    } else {
        path.split('/').map(|s| s.to_string()).collect()
    };

    let template = ProjectTreeTemplate {
        owner,
        project: &project,
        access_level: repo_context.project.access_level,
        ssh_clone_url: repo_context.ssh_clone_url,
        http_clone_url: repo_context.http_clone_url,
        summary: repo_context.summary,
        info,
        selected_branch,
        tree_entries,
        current_path: path,
        path_parts,
        readme_html,
        readme_frontmatter,
        parent_path,
        logged_in_user: repo_context.project.page.logged_in_user,
        sidebar_projects: repo_context.project.page.sidebar_projects,
        content_pages: repo_context.project.page.content_pages,
        active_tab: "code",
    };
    template.response()
}

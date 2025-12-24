use askama::Template;
use axum::{body::Body, extract::State, http::Response};
use gix::objs::tree::EntryKind;
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project, User,
    controllers::not_found,
    extractors::{PathUserProjectRef, PathUserProjectRefPath},
    models::ContentPage,
    services::{
        markdown::{self, Frontmatter},
        repository::{
            GitRefInfo, GitSummary, GitTreeEntry, get_git_file, get_git_info, get_git_summary,
            get_git_tree,
        },
        session,
    },
    views::ThemedRender,
};

#[derive(Template)]
#[template(path = "project_tree.html")]
struct ProjectTreeTemplate<'a> {
    owner: &'a User,
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
    logged_in_user: Option<&'a User>,
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
    owner: User,
    project: Project,
    git_ref: String,
    path: String,
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

    // Get tree entries
    let tree_result = get_git_tree(state, &owner.slug, &project.slug, &git_ref, &path).await;
    let tree_entries = match tree_result {
        Ok(entries) => entries,
        Err(_) => return not_found(logged_in_user, vec![]),
    };

    let git_user = logged_in_user
        .as_ref()
        .map(|u| u.slug.clone())
        .unwrap_or("anon".to_string());

    let ssh_clone_url = project.ssh_clone_url(&state.config.ssh_public_host, &git_user);
    let http_clone_url = project.http_clone_url(&state.config.base_url);

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
                .any(|e| e.filename == "README.md" && e.kind == EntryKind::Blob)
        })
        .map(|b| {
            let content = String::from_utf8_lossy(&b.data);
            let (frontmatter, html) = markdown::parse_and_render(&content);
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
        owner: &owner,
        project: &project,
        access_level,
        ssh_clone_url,
        http_clone_url,
        summary,
        info,
        selected_branch,
        tree_entries,
        current_path: path,
        path_parts,
        readme_html,
        readme_frontmatter,
        parent_path,
        logged_in_user: logged_in_user.as_ref(),
        sidebar_projects,
        content_pages: state.config.content_pages.clone(),
        active_tab: "code",
    };
    template.response()
}

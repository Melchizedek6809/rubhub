use askama::Template;
use axum::{
    body::Body,
    extract::{Query, State},
    http::{Response, StatusCode, header},
};
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project, User,
    controllers::not_found,
    extractors::PathUserProjectRefPath,
    models::ContentPage,
    services::{
        repository::{GitRefInfo, GitSummary, get_git_file, get_git_info, get_git_summary},
        session,
    },
    views::ThemedRender,
};

#[derive(Template)]
#[template(path = "project_blob.html")]
struct ProjectBlobTemplate<'a> {
    owner: &'a User,
    project: &'a Project,
    access_level: AccessType,
    ssh_clone_url: String,
    http_clone_url: String,
    selected_branch: String,
    summary: GitSummary,
    info: Option<GitRefInfo>,
    file_path: String,
    path_parts: Vec<String>,
    file_content: String,
    is_binary: bool,
    is_rendered_markdown: bool,
    raw_url: String,
    source_url: Option<String>,
    logged_in_user: Option<&'a User>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    active_tab: &'static str,
}

#[derive(Deserialize)]
pub struct BlobParams {
    /// Return raw file bytes with appropriate content-type
    pub raw: Option<String>,
    /// For markdown files, show raw markdown source in <pre><code>
    pub source: Option<String>,
}

pub async fn project_blob_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Query(params): Query<BlobParams>,
    PathUserProjectRefPath(owner, project, git_ref, path): PathUserProjectRefPath,
) -> Response<Body> {
    let logged_in_user = session::current_user(&state, &cookies).await.ok();

    let sidebar_projects = if let Some(ref user) = logged_in_user {
        user.sidebar_projects(&state).await
    } else {
        vec![]
    };

    let access_level = project
        .access_level(logged_in_user.as_ref().map(|user| user.slug.clone()))
        .await;

    if access_level == AccessType::None {
        return not_found(logged_in_user, vec![]);
    }

    let Some(summary) = get_git_summary(&state, &owner.slug, &project.slug).await else {
        return not_found(logged_in_user, vec![]);
    };

    // Get file content
    let file_result = get_git_file(&state, &owner.slug, &project.slug, &git_ref, &path).await;
    let file_obj = match file_result {
        Ok(obj) => obj,
        Err(_) => return not_found(logged_in_user, vec![]),
    };

    let git_user = logged_in_user
        .as_ref()
        .map(|u| u.slug.clone())
        .unwrap_or("anon".to_string());

    let ssh_clone_url = project.ssh_clone_url(&state.config.ssh_public_host, &git_user);
    let http_clone_url = project.http_clone_url(&state.config.base_url);

    let info = get_git_info(&state, &owner.slug, &project.slug, &git_ref, 1, 0).await;

    let selected_branch = info
        .as_ref()
        .map(|i| i.branch_name.to_string())
        .unwrap_or_else(|| git_ref.clone());

    // Detect file type
    let is_markdown = path.to_lowercase().ends_with(".md");

    // Build URLs for action links
    let raw_url = format!("{}?raw", project.uri_blob(&git_ref, &path));
    let source_url = if is_markdown {
        Some(format!("{}?source", project.uri_blob(&git_ref, &path)))
    } else {
        None
    };

    // Handle ?raw mode - return raw bytes with appropriate content-type
    if params.raw.is_some() {
        let mime_type = mime_guess::from_path(&path).first_or_octet_stream();
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime_type.as_ref())
            .body(Body::from(file_obj.data))
            .unwrap();
    }

    // Check if binary
    let is_binary = file_obj.data.contains(&0u8);

    // Handle binary files
    if is_binary {
        let path_parts: Vec<String> = path.split('/').map(|s| s.to_string()).collect();
        let template = ProjectBlobTemplate {
            owner: &owner,
            project: &project,
            access_level,
            ssh_clone_url,
            http_clone_url,
            summary,
            info,
            selected_branch,
            file_path: path.clone(),
            path_parts,
            file_content: String::from("Binary file"),
            is_binary: true,
            is_rendered_markdown: false,
            raw_url,
            source_url: None,
            logged_in_user: logged_in_user.as_ref(),
            sidebar_projects,
            content_pages: state.config.content_pages.clone(),
            active_tab: "code",
        };
        return template.response();
    }

    // Handle text files
    let text_content = String::from_utf8_lossy(&file_obj.data).to_string();

    // Render markdown if it's a .md file and ?source is not specified
    let (file_content, is_rendered_markdown) = if is_markdown && params.source.is_none() {
        let html = markdown::to_html_with_options(&text_content, &markdown::Options::gfm())
            .unwrap_or_else(|_| text_content.clone());
        (ammonia::clean(&html), true)
    } else {
        (text_content, false)
    };

    let path_parts: Vec<String> = path.split('/').map(|s| s.to_string()).collect();

    let template = ProjectBlobTemplate {
        owner: &owner,
        project: &project,
        access_level,
        ssh_clone_url,
        http_clone_url,
        summary,
        info,
        selected_branch,
        file_path: path.clone(),
        path_parts,
        file_content,
        is_binary: false,
        is_rendered_markdown,
        raw_url,
        source_url,
        logged_in_user: logged_in_user.as_ref(),
        sidebar_projects,
        content_pages: state.config.content_pages.clone(),
        active_tab: "code",
    };
    template.response()
}

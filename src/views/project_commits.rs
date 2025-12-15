use askama::Template;

use crate::{
    AccessType, Project, User,
    services::repository::{GitRefInfo, GitSummary},
    views::ThemedRender,
};

#[derive(Template)]
#[template(path = "project_commits.html")]
struct ProjectCommitsTemplate<'a> {
    owner: &'a User,
    project: &'a Project,
    selected_branch: String,
    access_level: AccessType,
    ssh_clone_url: String,
    summary: GitSummary,
    info: Option<GitRefInfo>,
    current_page: i32,
    page_count: i32,
    page_min: i32,
    page_max: i32,
}

pub async fn project_commits(
    owner: User,
    project: Project,
    access_level: AccessType,
    ssh_clone_url: String,
    summary: GitSummary,
    info: Option<GitRefInfo>,
    current_page: i32,
    page_count: i32,
) -> String {
    let selected_branch = info
        .as_ref()
        .map(|i| i.branch_name.to_string())
        .unwrap_or_default();

    let page_min = (current_page - 5).max(0);
    let page_max = (current_page + 5).min(page_count);

    ProjectCommitsTemplate {
        owner: &owner,
        project: &project,
        access_level,
        ssh_clone_url,
        summary,
        info,
        selected_branch,
        current_page,
        page_count,
        page_min,
        page_max,
    }
    .render_with_theme()
}

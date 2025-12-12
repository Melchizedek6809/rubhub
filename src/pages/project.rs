use axum::{
    extract::{Form, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project, ProjectSummary, User,
    extractors::{PathUser, PathUserProject, PathUserProjectBranch},
    services::{
        repository::{GitRefInfo, create_bare_repo, get_git_file, get_git_info, get_git_summary},
        session,
        validation::{validate_project_name, validate_uri},
    },
    views,
};

#[derive(Debug, Deserialize)]
pub struct NewProjectForm {
    pub name: String,
    pub public_access: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProjectSettingsForm {
    pub name: String,
    pub description: String,
    pub public_access: String,
    pub main_branch: String,
    pub website: String,
}

async fn not_found() -> (StatusCode, Html<String>) {
    (
        StatusCode::NOT_FOUND,
        Html(views::not_found::not_found().await),
    )
}

pub async fn project_list_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUser(owner): PathUser,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    let is_owner = session::current_user(&state, &cookies)
        .await
        .map(|user| user.id == owner.id)
        .unwrap_or(false);

    let projects = match owner.projects(&state).await {
        Ok(projects) => projects,
        Err(e) => {
            eprintln!("{:?}", e);
            return Err(not_found().await);
        }
    };

    let summaries: Vec<_> = projects
        .iter()
        .map(|p| ProjectSummary {
            name: p.name.as_str(),
            slug: p.slug.as_str(),
            owner_name: owner.name.as_str(),
            owner_slug: owner.slug.as_str(),
            description: p.description.as_str(),
        })
        .collect();

    Ok(Html(
        views::user::profile(&owner, &summaries, is_owner).await,
    ))
}

pub async fn new_project_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
) -> Result<Html<String>, Redirect> {
    match session::current_user(&state, &cookies).await {
        Ok(_) => Ok(render_new_project_page(&cookies, None, AccessType::Read).await),
        Err(_) => Err(Redirect::to("/login")),
    }
}

fn parse_public_access(value: &str) -> Result<AccessType, &'static str> {
    match value.to_ascii_lowercase().as_str() {
        "none" => Ok(AccessType::None),
        "read" => Ok(AccessType::Read),
        "write" => Ok(AccessType::Write),
        "admin" => Err("Public admin access is not allowed."),
        _ => Err("Invalid access level."),
    }
}

pub async fn handle_new_project(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Form(form): Form<NewProjectForm>,
) -> Result<Response, Redirect> {
    let selected_public_access = form
        .public_access
        .as_deref()
        .and_then(|value| parse_public_access(value).ok())
        .unwrap_or(AccessType::Read);

    let current_user = match session::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Err(Redirect::to("/login")),
    };

    let name = form.name.trim();
    if name.is_empty() {
        return Ok(render_new_project_page(
            &cookies,
            Some("Name is required."),
            selected_public_access,
        )
        .await
        .into_response());
    }

    if let Err(msg) = validate_project_name(name) {
        return Ok(
            render_new_project_page(&cookies, Some(msg), selected_public_access)
                .await
                .into_response(),
        );
    }

    let user_slug = current_user.slug.clone();

    match Project::new(&current_user, name, selected_public_access) {
        Ok(project) => {
            if let Ok(_) = Project::load(&state, &project.owner, &project.slug).await {
                return Ok(render_new_project_page(
                    &cookies,
                    Some("Project already exists"),
                    selected_public_access,
                )
                .await
                .into_response());
            };
            match project.save(&state).await {
                Ok(_) => {
                    match create_bare_repo(&state, user_slug.clone(), project.slug.clone()).await {
                        Ok(_) => Ok(Redirect::to(&project.uri()).into_response()),
                        Err(_) => Ok(render_new_project_page(
                            &cookies,
                            Some("Could not create project."),
                            selected_public_access,
                        )
                        .await
                        .into_response()),
                    }
                }
                Err(msg) => Ok(render_new_project_page(
                    &cookies,
                    Some(&msg.to_string()),
                    selected_public_access,
                )
                .await
                .into_response()),
            }
        }
        Err(msg) => {
            Ok(
                render_new_project_page(&cookies, Some(&msg.to_string()), selected_public_access)
                    .await
                    .into_response(),
            )
        }
    }
}

pub async fn render_project_page(
    state: &GlobalState,
    cookies: Cookies,
    owner: User,
    project: Project,
    branch: Option<String>,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    let Some(summary) = get_git_summary(&state, &owner.slug, &project.slug).await else {
        return Err(not_found().await);
    };

    let current = match branch {
        Some(branch) => branch,
        None => project.main_branch.clone(),
    };
    let info = get_git_info(&state, &owner.slug, &project.slug, &current, 1, 0).await;

    let session_user = session::current_user(&state, &cookies).await.ok();
    let access_level = project
        .access_level(session_user.as_ref().map(|user| user.slug.clone()))
        .await;
    let git_user = session_user.map(|u| u.slug).unwrap_or("anon".to_string());

    let ssh_clone_url = format!(
        "ssh://{}@{}/{}/{}",
        git_user, state.config.ssh_public_host, owner.slug, project.slug
    );

    let readme = get_git_file(&state, &owner.slug, &project.slug, &current, "README.md").await;
    let readme = readme
        .map(|b| {
            let str = String::from_utf8_lossy(&b.data);
            let html =
                markdown::to_html_with_options(&str, &markdown::Options::gfm()).unwrap_or_default();

            ammonia::clean(&html)
        })
        .ok();

    Ok(Html(
        views::project::project_with_access(
            owner,
            project,
            access_level,
            ssh_clone_url,
            summary,
            info,
            readme,
        )
        .await,
    ))
}

pub async fn project_page_tree(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProjectBranch(owner, project, branch): PathUserProjectBranch,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    render_project_page(&state, cookies, owner, project, Some(branch)).await
}

pub async fn project_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, project): PathUserProject,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    render_project_page(&state, cookies, owner, project, None).await
}

pub async fn project_page_branches(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, project): PathUserProject,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    let Some(summary) = get_git_summary(&state, &owner.slug, &project.slug).await else {
        return Err(not_found().await);
    };

    let mut branches: Vec<GitRefInfo> = vec![];
    for b in &summary.branches {
        if let Some(info) = get_git_info(&state, &owner.slug, &project.slug, b, 1, 0).await {
            branches.push(info);
        }
    }

    let session_user = session::current_user(&state, &cookies).await.ok();
    let access_level = project
        .access_level(session_user.as_ref().map(|user| user.slug.clone()))
        .await;

    Ok(Html(
        views::project_branches::project_branches(owner, project, access_level, branches).await,
    ))
}

pub async fn project_page_tags(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, project): PathUserProject,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    let Some(summary) = get_git_summary(&state, &owner.slug, &project.slug).await else {
        return Err(not_found().await);
    };

    let mut tags: Vec<GitRefInfo> = vec![];
    for b in &summary.tags {
        if let Some(info) = get_git_info(&state, &owner.slug, &project.slug, b, 1, 0).await {
            tags.push(info);
        }
    }

    let session_user = session::current_user(&state, &cookies).await.ok();
    let access_level = project
        .access_level(session_user.as_ref().map(|user| user.slug.clone()))
        .await;

    Ok(Html(
        views::project_tags::project_tags(owner, project, access_level, tags).await,
    ))
}

#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub page: Option<usize>,
}

pub async fn project_page_commits(
    State(state): State<GlobalState>,
    cookies: Cookies,
    Query(q): Query<Pagination>,
    PathUserProjectBranch(owner, project, current): PathUserProjectBranch,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    let Some(summary) = get_git_summary(&state, &owner.slug, &project.slug).await else {
        return Err(not_found().await);
    };

    let page_size: usize = 20;
    let current_page = q.page.unwrap_or(0);
    let offset = current_page * page_size;
    let info = get_git_info(
        &state,
        &owner.slug,
        &project.slug,
        &current,
        page_size,
        offset,
    )
    .await;
    let commit_count = info.as_ref().map(|i| i.commit_count).unwrap_or(0);
    let page_count = commit_count / page_size;

    let session_user = session::current_user(&state, &cookies).await.ok();
    let access_level = project
        .access_level(session_user.as_ref().map(|user| user.slug.clone()))
        .await;
    let git_user = session_user.map(|u| u.slug).unwrap_or("anon".to_string());

    let ssh_clone_url = format!(
        "ssh://{}@{}/{}/{}",
        git_user, state.config.ssh_public_host, owner.slug, project.slug
    );

    Ok(Html(
        views::project_commits::project_commits(
            owner,
            project,
            access_level,
            ssh_clone_url,
            summary,
            info,
            current_page.try_into().unwrap_or_default(),
            page_count.try_into().unwrap_or(1),
        )
        .await,
    ))
}

pub async fn project_settings_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, project): PathUserProject,
) -> Result<Html<String>, Redirect> {
    let current_user = match session::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Err(Redirect::to("/login")),
    };

    let access_level = project.access_level(Some(current_user.slug.clone())).await;
    if access_level != AccessType::Admin {
        return Err(Redirect::to(&project.uri()));
    }

    Ok(render_project_settings_page(&cookies, owner, project, None).await)
}

pub async fn handle_project_settings(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, mut project): PathUserProject,
    Form(form): Form<ProjectSettingsForm>,
) -> Result<Response, Redirect> {
    let current_user = match session::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Err(Redirect::to("/login")),
    };

    let access_level = project.access_level(Some(current_user.slug.clone())).await;
    if access_level != AccessType::Admin {
        return Err(Redirect::to(&project.uri()));
    }

    let name = form.name.trim();
    let description = form.description.trim();
    let main_branch = form.main_branch.trim();
    let website = form.website.trim();

    let public_access = match parse_public_access(&form.public_access) {
        Ok(level) => level,
        Err(msg) => {
            return Ok(
                render_project_settings_page(&cookies, owner, project, Some(msg))
                    .await
                    .into_response(),
            );
        }
    };

    if let Err(msg) = validate_project_name(name) {
        return Ok(
            render_project_settings_page(&cookies, owner, project, Some(msg))
                .await
                .into_response(),
        );
    }

    if !website.is_empty()
        && let Err(msg) = validate_uri(website)
    {
        return Ok(
            render_project_settings_page(&cookies, owner, project, Some(msg))
                .await
                .into_response(),
        );
    }

    if name.is_empty() {
        return Ok(render_project_settings_page(
            &cookies,
            owner,
            project,
            Some("Name is required."),
        )
        .await
        .into_response());
    }
    if main_branch.is_empty() {
        return Ok(render_project_settings_page(
            &cookies,
            owner,
            project,
            Some("Branch name is required."),
        )
        .await
        .into_response());
    }

    project.name = name.to_owned();
    project.public_access = public_access;
    project.description = description.to_owned();
    project.main_branch = main_branch.to_owned();
    project.website = website.to_owned();

    if project.save(&state).await.is_err() {
        // A proper error message would be nicer here
        return Err(Redirect::to(&project.uri()));
    }

    Ok(Redirect::to(&project.uri_settings()).into_response())
}

async fn render_new_project_page(
    _cookies: &tower_cookies::Cookies,
    message: Option<&str>,
    public_access: AccessType,
) -> Html<String> {
    Html(views::project_new::new_project(message, public_access).await)
}

async fn render_project_settings_page(
    _cookies: &tower_cookies::Cookies,
    owner: User,
    project: Project,
    message: Option<&str>,
) -> Html<String> {
    Html(views::project_settings::project_settings(owner, project, message).await)
}

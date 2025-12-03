use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use tokio::{fs, process::Command};
use uuid::Uuid;

use crate::{
    app::{self, ProjectSummary},
    entities::{AccessType, project, user},
    services::session,
    state::GlobalState,
};

#[derive(Debug, Deserialize)]
pub struct NewProjectForm {
    pub name: String,
}

pub async fn projects_page(
    state: &GlobalState,
    cookies: tower_cookies::Cookies,
    username: String,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    let Some(owner) = user::Entity::find()
        .filter(user::Column::Name.eq(username.clone()))
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Err(not_found().await);
    };

    let is_owner = session::current_user(state, &cookies)
        .await
        .map(|user| user.id == owner.id)
        .unwrap_or(false);

    let projects = project::Entity::find()
        .filter(project::Column::Owner.eq(owner.id))
        .all(&state.db)
        .await
        .unwrap_or_default();

    let summaries: Vec<_> = projects
        .iter()
        .map(|p| ProjectSummary {
            name: p.name.as_str(),
            slug: p.slug.as_str(),
            owner: owner.name.as_str(),
        })
        .collect();

    Ok(Html(app::projects(&owner.name, &summaries, is_owner).await))
}

pub async fn new_project_page(
    state: &GlobalState,
    cookies: tower_cookies::Cookies,
) -> Result<Html<String>, Redirect> {
    match session::current_user(state, &cookies).await {
        Ok(_) => Ok(Html(app::new_project(None).await)),
        Err(_) => Err(Redirect::to("/login")),
    }
}

pub async fn create_bare_repo(
    state: &GlobalState,
    user: String,
    project: String,
) -> Result<(), std::io::Error> {
    let path = state.config.git_root.join(user);
    fs::create_dir_all(&path).await?;

    let path = path.join(project);
    let status = Command::new("git")
        .arg("init")
        .arg("--bare")
        .arg(path)
        .kill_on_drop(true) // makes shutdowns cleaner
        .status()
        .await?;

    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other("git init --bare failed"))
    }
}

pub async fn handle_new_project(
    state: &GlobalState,
    cookies: tower_cookies::Cookies,
    form: NewProjectForm,
) -> Result<Response, Redirect> {
    let current_user = match session::current_user(state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Err(Redirect::to("/login")),
    };

    let name = form.name.trim();
    if name.is_empty() {
        return Ok(Html(app::new_project(Some("Name is required.")).await).into_response());
    }

    let slug = generate_unique_slug(state, name, current_user.id).await;
    let username = current_user.name.clone();

    let new_project = project::ActiveModel {
        id: Set(Uuid::new_v4()),
        owner: Set(current_user.id),
        slug: Set(slug),
        name: Set(name.to_owned()),
        description: Set(String::new()),
        default_access: Set(Some(AccessType::None)),
        public_access: Set(AccessType::None),
        meta: Set(serde_json::json!({})),
        ..Default::default()
    };

    match new_project.insert(&state.db).await {
        Ok(_) => {
            let res = create_bare_repo(state, username.clone(), name.to_owned()).await;
            if res.is_err() {
                Ok(Html(app::new_project(Some("Could not create project.")).await).into_response())
            } else {
                Ok(Redirect::to(&format!("/{username}/projects")).into_response())
            }
        }
        Err(_) => {
            Ok(Html(app::new_project(Some("Could not create project.")).await).into_response())
        }
    }
}

pub async fn project_page(
    state: &GlobalState,
    cookies: tower_cookies::Cookies,
    username: String,
    slug: String,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    let Some(owner) = user::Entity::find()
        .filter(user::Column::Name.eq(username.clone()))
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Err(not_found().await);
    };

    let project = project::Entity::find()
        .filter(project::Column::Owner.eq(owner.id))
        .filter(project::Column::Slug.eq(slug.clone()))
        .one(&state.db)
        .await
        .ok()
        .flatten();

    let Some(project) = project else {
        return Err(not_found().await);
    };

    let session_user = session::current_user(state, &cookies).await.ok();
    let access_level =
        project_access_level(state, session_user.as_ref().map(|user| user.id), project.id).await;
    let can_manage = matches!(access_level, AccessType::Admin);

    Ok(Html(
        app::project_with_access(
            &project.name,
            &project.slug,
            &owner.name,
            access_level,
            can_manage,
        )
        .await,
    ))
}

#[derive(Debug, Deserialize)]
pub struct ProjectSettingsForm {
    pub name: String,
    pub public_access: String,
}

pub async fn project_settings_page(
    state: &GlobalState,
    cookies: tower_cookies::Cookies,
    username: String,
    slug: String,
) -> Result<Html<String>, Redirect> {
    let Some(owner) = user::Entity::find()
        .filter(user::Column::Name.eq(username.clone()))
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Err(Redirect::to("/"));
    };

    let Some(project) = project::Entity::find()
        .filter(project::Column::Owner.eq(owner.id))
        .filter(project::Column::Slug.eq(slug.clone()))
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Err(Redirect::to(&format!("/{username}/projects")));
    };

    let current_user = match session::current_user(state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Err(Redirect::to("/login")),
    };

    let access_level = project_access_level(state, Some(current_user.id), project.id).await;
    if access_level != AccessType::Admin {
        return Err(Redirect::to(&format!("/{username}/{slug}")));
    }

    Ok(Html(
        app::project_settings(
            &project.name,
            &project.slug,
            &current_user.name,
            project.public_access,
            None,
        )
        .await,
    ))
}

pub async fn handle_project_settings(
    state: &GlobalState,
    cookies: tower_cookies::Cookies,
    username: String,
    slug: String,
    form: ProjectSettingsForm,
) -> Result<Response, Redirect> {
    let Some(owner) = user::Entity::find()
        .filter(user::Column::Name.eq(username.clone()))
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Err(Redirect::to("/"));
    };

    let Some(mut project) = project::Entity::find()
        .filter(project::Column::Owner.eq(owner.id))
        .filter(project::Column::Slug.eq(slug.clone()))
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Err(Redirect::to(&format!("/{username}/projects")));
    };

    let current_user = match session::current_user(state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Err(Redirect::to("/login")),
    };

    let access_level = project_access_level(state, Some(current_user.id), project.id).await;
    if access_level != AccessType::Admin {
        return Err(Redirect::to(&format!("/{username}/{slug}")));
    }

    let name = form.name.trim();
    let public_access = match parse_public_access(&form.public_access) {
        Ok(level) => level,
        Err(msg) => {
            return Ok(Html(
                app::project_settings(
                    name,
                    &project.slug,
                    &current_user.name,
                    project.public_access,
                    Some(msg),
                )
                .await,
            )
            .into_response());
        }
    };
    if name.is_empty() {
        return Ok(Html(
            app::project_settings(
                name,
                &project.slug,
                &current_user.name,
                project.public_access,
                Some("Name is required."),
            )
            .await,
        )
        .into_response());
    }

    project.name = name.to_owned();
    project.public_access = public_access;
    let mut active: project::ActiveModel = project.into();
    active.name = Set(name.to_owned());
    active.public_access = Set(public_access);

    if active.update(&state.db).await.is_err() {
        return Ok(Html(
            app::project_settings(
                name,
                &slug,
                &current_user.name,
                public_access,
                Some("Could not update project."),
            )
            .await,
        )
        .into_response());
    }

    Ok(Redirect::to(&format!("/{username}/{slug}/settings")).into_response())
}

async fn not_found() -> (StatusCode, Html<String>) {
    (StatusCode::NOT_FOUND, Html(app::not_found().await))
}

pub async fn project_access_level(
    state: &GlobalState,
    user_id: Option<Uuid>,
    project_id: Uuid,
) -> AccessType {
    let Some(project) = project::Entity::find_by_id(project_id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return AccessType::None;
    };

    project_access_level_for(&project, user_id)
}

fn project_access_level_for(project: &project::Model, user_id: Option<Uuid>) -> AccessType {
    if let Some(uid) = user_id && uid == project.owner {
        return AccessType::Admin;
    }

    project.public_access
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

async fn generate_unique_slug(state: &GlobalState, name: &str, owner: Uuid) -> String {
    let base = slugify(name);
    let mut slug = base.clone();

    for _ in 0..5 {
        let exists = project::Entity::find()
            .filter(project::Column::Slug.eq(slug.clone()))
            .one(&state.db)
            .await
            .ok()
            .flatten();

        if exists.is_none() {
            return slug;
        }

        let short = Uuid::new_v4().to_string();
        let short = short.get(..8).unwrap_or(&short);
        slug = format!("{base}-{short}");
    }

    format!("{base}-{}", owner.to_string().get(..8).unwrap_or("project"))
}

fn slugify(name: &str) -> String {
    let mut result = String::new();
    let mut last_dash = false;

    for ch in name.chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            result.push(lower);
            last_dash = false;
        } else if !last_dash {
            result.push('-');
            last_dash = true;
        }
    }

    while result.starts_with('-') {
        result.remove(0);
    }
    while result.ends_with('-') {
        result.pop();
    }

    if result.is_empty() {
        "project".to_owned()
    } else {
        result
    }
}

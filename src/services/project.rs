use axum::response::{Html, IntoResponse, Redirect, Response};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    app::{self, ProjectSummary},
    entities::{AccessType, project},
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
) -> Result<Html<String>, Redirect> {
    let current_user = match session::current_user(state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Err(Redirect::to("/login")),
    };

    let projects = project::Entity::find()
        .filter(project::Column::Owner.eq(current_user.id))
        .all(&state.db)
        .await
        .unwrap_or_default();

    let summaries: Vec<_> = projects
        .iter()
        .map(|p| ProjectSummary {
            name: p.name.as_str(),
            slug: p.slug.as_str(),
        })
        .collect();

    Ok(Html(app::projects(&summaries).await))
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

    let new_project = project::ActiveModel {
        id: Set(Uuid::new_v4()),
        owner: Set(current_user.id),
        slug: Set(slug),
        name: Set(name.to_owned()),
        description: Set(String::new()),
        default_access: Set(Some(AccessType::None)),
        meta: Set(serde_json::json!({})),
        ..Default::default()
    };

    match new_project.insert(&state.db).await {
        Ok(_) => Ok(Redirect::to("/projects").into_response()),
        Err(_) => {
            Ok(Html(app::new_project(Some("Could not create project.")).await).into_response())
        }
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

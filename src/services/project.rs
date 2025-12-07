use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::{
    entities::{AccessType, project, user},
    services::validation::{slugify, validate_slug},
    state::GlobalState,
};

pub async fn find_project_by_path(
    state: &GlobalState,
    path: &str,
) -> Option<(project::Model, user::Model)> {
    let trimmed = path.trim_matches('/');
    let mut parts = trimmed.split('/');
    let username = parts.next()?.trim();
    let mut slug = parts.next()?.trim();

    if parts.next().is_some() || username.is_empty() || slug.is_empty() {
        return None;
    }

    if let Some(stripped) = slug.strip_suffix(".git") {
        slug = stripped;
    }

    if validate_slug(username).is_err() || validate_slug(slug).is_err() {
        return None;
    }

    let query = project::Entity::find()
        .find_also_related(user::Entity)
        .filter(project::Column::Slug.eq(slug))
        .filter(user::Column::Name.eq(username));

    let (project, owner) = (query.one(&state.db).await.ok()?)?;

    Some((project, owner?))
}

pub async fn get_project(
    db: &DatabaseConnection,
    user_id: Uuid,
    project_slug: String,
) -> Option<project::Model> {
    project::Entity::find()
        .filter(project::Column::Owner.eq(user_id))
        .filter(project::Column::Slug.eq(project_slug))
        .one(db)
        .await
        .ok()
        .flatten()
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

pub fn project_access_level_for(project: &project::Model, user_id: Option<Uuid>) -> AccessType {
    if let Some(uid) = user_id
        && uid == project.owner
    {
        return AccessType::Admin;
    }

    project.public_access
}

pub fn parse_public_access(value: &str) -> Result<AccessType, &'static str> {
    match value.to_ascii_lowercase().as_str() {
        "none" => Ok(AccessType::None),
        "read" => Ok(AccessType::Read),
        "write" => Ok(AccessType::Write),
        "admin" => Err("Public admin access is not allowed."),
        _ => Err("Invalid access level."),
    }
}

pub async fn generate_unique_slug(state: &GlobalState, name: &str, owner: Uuid) -> String {
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

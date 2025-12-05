use axum::{extract::State, response::Html};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use crate::{
    app::{self, ProjectSummary},
    entities::{AccessType, project, user},
    state::GlobalState,
};

pub async fn index(State(state): State<GlobalState>) -> Html<String> {
    let projects = project::Entity::find()
        .filter(project::Column::PublicAccess.ne(AccessType::None))
        .order_by_desc(project::Column::CreatedAt)
        .limit(3)
        .find_also_related(user::Entity)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let featured: Vec<ProjectSummary<'_>> = projects
        .iter()
        .filter_map(|(project, owner)| {
            owner.as_ref().map(|owner| ProjectSummary {
                name: project.name.as_str(),
                slug: project.slug.as_str(),
                owner: owner.name.as_str(),
                description: project.description.as_str(),
            })
        })
        .collect();

    Html(app::index(&featured).await)
}

use axum::{extract::State, response::Html};

use crate::{
    app::{self, ProjectSummary},
    state::GlobalState,
};

pub async fn index(State(_state): State<GlobalState>) -> Html<String> {
    let featured: Vec<ProjectSummary<'_>> = vec![];

    Html(app::index(&featured).await)
}

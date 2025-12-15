use askama::Template;

use crate::{ProjectSummary, views::ThemedRender};

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    featured: &'a [ProjectSummary<'a>],
}

pub async fn index(featured: &[ProjectSummary<'_>]) -> String {
    IndexTemplate { featured }.render_with_theme()
}

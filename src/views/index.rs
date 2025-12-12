use askama::Template;

use crate::{
    ProjectSummary,
    views::{extract_html_parts, theme_render},
};

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    featured: &'a [ProjectSummary<'a>],
}

pub async fn index(featured: &[ProjectSummary<'_>]) -> String {
    let contents = IndexTemplate { featured }.render().unwrap();
    let (head, body) = extract_html_parts(&contents);
    theme_render(head, body).await
}

use askama::Template;
use axum::{extract::State, http::StatusCode, response::Html};
use tower_cookies::Cookies;

use crate::{
    GlobalState, ProjectSummary, User, controllers::not_found, extractors::PathUser,
    services::session, views::ThemedRender,
};

#[derive(Template)]
#[template(path = "user.html")]
struct UserTemplate<'a> {
    user: &'a User,
    projects: &'a [ProjectSummary<'a>],
    is_owner: bool,
    logged_in_user: Option<&'a User>,
}

pub async fn user_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUser(owner): PathUser,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    let logged_in_user = session::current_user(&state, &cookies).await.ok();
    let is_owner = logged_in_user
        .as_ref()
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

    let template = UserTemplate {
        user: &owner,
        projects: &summaries,
        is_owner,
        logged_in_user: logged_in_user.as_ref(),
    };
    Ok(Html(template.render_with_theme()))
}

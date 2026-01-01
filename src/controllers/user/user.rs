use std::sync::Arc;

use askama::Template;
use axum::{body::Body, extract::State, http::Response, response::Html};
use tower_cookies::Cookies;

use crate::{
    GlobalState, Project, ProjectSummary, User, controllers::not_found, extractors::PathUser, UserModel,
    models::ContentPage, services::session, views::ThemedRender,
};

#[derive(Template)]
#[template(path = "user.html")]
struct UserTemplate<'a> {
    user: &'a User,
    projects: &'a [ProjectSummary<'a>],
    is_owner: bool,
    logged_in_user: Option<Arc<User>>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
}

pub async fn user_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUser(owner): PathUser,
) -> Result<Html<String>, Response<Body>> {
    let logged_in_user = session::current_user(&state, &cookies).await.ok();

    let sidebar_projects = if let Some(ref user) = logged_in_user {
        user.sidebar_projects(&state).await
    } else {
        vec![]
    };

    let is_owner = logged_in_user
        .as_ref()
        .map(|user| user.slug == owner.slug)
        .unwrap_or(false);

    let projects = match owner.projects(&state).await {
        Ok(projects) => projects,
        Err(e) => {
            eprintln!("{:?}", e);
            return Err(not_found(
                logged_in_user,
                state.config.content_pages.clone(),
            ));
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
        logged_in_user,
        sidebar_projects,
        content_pages: state.config.content_pages.clone(),
    };
    Ok(Html(template.render_with_theme()))
}

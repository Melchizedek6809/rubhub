use std::sync::Arc;

use askama::Template;
use axum::{body::Body, extract::State, http::Response, response::Html};
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project, ProjectSummary, User, UserModel,
    controllers::context::PageContext, extractors::PathUser, models::ContentPage,
    services::meta::PageMeta, views::ThemedRender,
};

#[derive(Template)]
#[template(path = "user/profile.html")]
struct UserTemplate<'a> {
    user: &'a User,
    projects: &'a [ProjectSummary<'a>],
    is_owner: bool,
    logged_in_user: Option<Arc<User>>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    meta: PageMeta,
}

pub async fn user_page(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUser(owner): PathUser,
) -> Result<Html<String>, Response<Body>> {
    let page_context = PageContext::load(&state, &cookies).await;

    let is_owner = page_context
        .logged_in_user
        .as_ref()
        .map(|user| user.slug == owner.slug)
        .unwrap_or(false);

    let projects = match owner.projects(&state).await {
        Ok(projects) => projects,
        Err(e) => {
            eprintln!("{:?}", e);
            return Err(page_context.not_found());
        }
    };

    let visible_projects: Vec<_> = projects
        .iter()
        .filter(|project| is_owner || project.public_access != AccessType::None)
        .collect();

    let summaries: Vec<_> = visible_projects
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
        logged_in_user: page_context.logged_in_user,
        sidebar_projects: page_context.sidebar_projects,
        content_pages: page_context.content_pages,
        meta: PageMeta::new(
            format!("{} - RubHub", owner.name),
            format!(
                "{} has {} public project{} on RubHub.",
                owner.name,
                summaries.len(),
                if summaries.len() == 1 { "" } else { "s" }
            ),
        )
        .canonical(&state.config.base_url, &owner.uri()),
    };
    Ok(Html(template.render_with_theme()))
}

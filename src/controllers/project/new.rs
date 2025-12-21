use askama::Template;
use axum::{
    body::Body,
    extract::State,
    http::Response,
    response::{Html, IntoResponse, Redirect},
};
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project, User,
    extractors::CsrfForm,
    models::ContentPage,
    services::{csrf, repository::create_bare_repo, session, validation::validate_project_name},
    views::ThemedRender,
};

#[derive(Debug, Deserialize)]
pub struct NewProjectForm {
    pub name: String,
    pub public_access: Option<String>,
}

#[derive(Template)]
#[template(path = "project_new.html")]
struct NewProjectTemplate<'a> {
    message: Option<&'a str>,
    logged_in_user: Option<&'a User>,
    sidebar_projects: Vec<Project>,
    content_pages: Vec<ContentPage>,
    csrf_token_field: String,
}

pub async fn project_new_get(
    State(state): State<GlobalState>,
    cookies: Cookies,
) -> Result<Html<String>, Redirect> {
    let logged_in_user = match session::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Err(Redirect::to("/login")),
    };
    Ok(render_new_project_page(&state, &cookies, Some(&logged_in_user), None).await)
}

async fn render_new_project_page(
    state: &GlobalState,
    cookies: &Cookies,
    logged_in_user: Option<&User>,
    message: Option<&str>,
) -> Html<String> {
    let sidebar_projects = if let Some(user) = logged_in_user {
        user.sidebar_projects(state).await
    } else {
        vec![]
    };

    let token = csrf::get_or_create_token(&state.config.csrf_secret, cookies);
    let csrf_token_field = csrf::hidden_field(&token);

    let template = NewProjectTemplate {
        message,
        logged_in_user,
        sidebar_projects,
        content_pages: state.config.content_pages.clone(),
        csrf_token_field,
    };
    Html(template.render_with_theme())
}

pub async fn project_new_post(
    State(state): State<GlobalState>,
    cookies: Cookies,
    CsrfForm(form): CsrfForm<NewProjectForm>,
) -> Response<Body> {
    let selected_public_access = form
        .public_access
        .as_deref()
        .and_then(|value| AccessType::parse_public_access(value).ok())
        .unwrap_or(AccessType::Read);

    let current_user = match session::current_user(&state, &cookies).await {
        Ok(user) => user,
        Err(_) => return Redirect::to("/login").into_response(),
    };

    let name = form.name.trim();
    if name.is_empty() {
        return render_new_project_page(
            &state,
            &cookies,
            Some(&current_user),
            Some("Name is required."),
        )
        .await
        .into_response();
    }

    if let Err(msg) = validate_project_name(name) {
        return render_new_project_page(&state, &cookies, Some(&current_user), Some(msg))
            .await
            .into_response();
    }

    let user_slug = current_user.slug.clone();

    match Project::new(&current_user, name, selected_public_access) {
        Ok(project) => {
            if Project::load(&state, &project.owner, &project.slug)
                .await
                .is_ok()
            {
                return render_new_project_page(
                    &state,
                    &cookies,
                    Some(&current_user),
                    Some("Project already exists"),
                )
                .await
                .into_response();
            };
            match project.save(&state).await {
                Ok(_) => {
                    match create_bare_repo(&state, user_slug.clone(), project.slug.clone()).await {
                        Ok(_) => Redirect::to(&project.uri()).into_response(),
                        Err(_) => render_new_project_page(
                            &state,
                            &cookies,
                            Some(&current_user),
                            Some("Could not create project."),
                        )
                        .await
                        .into_response(),
                    }
                }
                Err(msg) => render_new_project_page(
                    &state,
                    &cookies,
                    Some(&current_user),
                    Some(&msg.to_string()),
                )
                .await
                .into_response(),
            }
        }
        Err(msg) => render_new_project_page(
            &state,
            &cookies,
            Some(&current_user),
            Some(&msg.to_string()),
        )
        .await
        .into_response(),
    }
}

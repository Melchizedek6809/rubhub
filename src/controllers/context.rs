use std::sync::Arc;

use axum::{
    body::Body,
    http::Response,
    response::{IntoResponse, Redirect},
};
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project, User, UserModel,
    models::ContentPage,
    services::{repository::GitSummary, session},
};

pub(crate) struct PageContext {
    pub logged_in_user: Option<Arc<User>>,
    pub sidebar_projects: Vec<Project>,
    pub content_pages: Vec<ContentPage>,
}

impl PageContext {
    pub async fn load(state: &GlobalState, cookies: &Cookies) -> Self {
        let logged_in_user = session::current_user(state, cookies).await.ok();
        let sidebar_projects = if let Some(ref user) = logged_in_user {
            user.sidebar_projects(state).await
        } else {
            vec![]
        };

        Self {
            logged_in_user,
            sidebar_projects,
            content_pages: state.config.content_pages.clone(),
        }
    }

    pub fn not_found(self) -> Response<Body> {
        crate::controllers::not_found(
            self.logged_in_user,
            self.sidebar_projects,
            self.content_pages,
        )
    }
}

pub(crate) async fn require_user(
    state: &GlobalState,
    cookies: &Cookies,
) -> Result<Arc<User>, Response<Body>> {
    session::current_user(state, cookies)
        .await
        .map_err(|_| Redirect::to("/login").into_response())
}

pub(crate) struct ProjectPageContext {
    pub page: PageContext,
    pub access_level: AccessType,
}

impl ProjectPageContext {
    pub async fn load(
        state: &GlobalState,
        cookies: &Cookies,
        project: &Project,
    ) -> Result<Self, Response<Body>> {
        let page = PageContext::load(state, cookies).await;
        let access_level = project
            .access_level(page.logged_in_user.as_ref().map(|user| user.slug.clone()))
            .await;

        if access_level == AccessType::None {
            return Err(page.not_found());
        }

        Ok(Self { page, access_level })
    }

    pub fn not_found(self) -> Response<Body> {
        self.page.not_found()
    }
}

pub(crate) struct ProjectRepoContext {
    pub project: ProjectPageContext,
    pub summary: GitSummary,
    pub ssh_clone_url: String,
    pub http_clone_url: String,
}

impl ProjectRepoContext {
    pub async fn load(
        state: &GlobalState,
        cookies: &Cookies,
        owner_slug: &str,
        project: &Project,
    ) -> Result<Self, Response<Body>> {
        let project_context = ProjectPageContext::load(state, cookies, project).await?;
        let Some(summary) =
            crate::services::repository::get_git_summary(state, owner_slug, &project.slug).await
        else {
            return Err(project_context.not_found());
        };

        Ok(Self {
            project: project_context,
            summary,
            ssh_clone_url: project.ssh_clone_url(&state.config.ssh_public_host),
            http_clone_url: project.http_clone_url(&state.config.base_url),
        })
    }
}

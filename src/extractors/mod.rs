pub mod path_user;
pub mod path_user_project;
pub mod path_user_project_branch;
pub mod path_user_project_ref_path;

use axum::{
    body::Body,
    extract::{FromRef, FromRequestParts},
    http::{Response, request::Parts},
};
use tower_cookies::Cookies;

use crate::{GlobalState, UserModel, controllers, services::session};

pub use path_user::PathUser;
pub use path_user_project::PathUserProject;
pub use path_user_project_branch::PathUserProjectBranch;
pub use path_user_project_ref_path::{PathUserProjectRef, PathUserProjectRefPath};

pub(crate) async fn themed_not_found<S>(parts: &mut Parts, state: &S) -> Response<Body>
where
    S: Send + Sync,
    GlobalState: FromRef<S>,
{
    let app_state = GlobalState::from_ref(state);
    let content_pages = app_state.config.content_pages.clone();

    let cookies = Cookies::from_request_parts(parts, state).await.ok();
    let logged_in_user = match cookies {
        Some(cookies) => session::current_user(&app_state, &cookies).await.ok(),
        None => None,
    };

    let sidebar_projects = if let Some(ref user) = logged_in_user {
        user.sidebar_projects(&app_state).await
    } else {
        vec![]
    };

    controllers::not_found(logged_in_user, sidebar_projects, content_pages)
}

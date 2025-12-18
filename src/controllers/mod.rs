mod auth;
mod content_page;
mod landing;
mod not_found;
mod project;
mod user;

pub use auth::{handle_login, handle_registration, login_page, logout, registration_page};
pub use content_page::render_content_page;
pub use landing::index;
pub use not_found::{not_found, not_found_get};
pub use project::{
    git_info_refs, git_upload_pack, project_branches_get, project_commits_get,
    project_delete_post, project_new_get, project_new_post, project_overview_get,
    project_overview_tree_get, project_settings_get, project_settings_post, project_tags_get,
};
pub use user::{handle_settings, settings_page, user_page};

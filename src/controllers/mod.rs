mod auth;
mod content_page;
mod landing;
mod not_found;
mod project;
mod projects_list;
mod user;

pub use auth::{handle_login, handle_registration, login_page, logout, registration_page};
pub use content_page::render_content_page;
pub use landing::index;
pub use not_found::{not_found, not_found_get};
pub use project::{
    git_info_refs, git_upload_pack, issue_comment_post, issue_new_get, issue_new_post,
    issue_view_get, issues_list_get, project_blob_get, project_branches_get, project_commits_get,
    project_delete_post, project_new_get, project_new_post, project_overview_get,
    project_settings_get, project_settings_post, project_tags_get, project_tree_get,
    project_tree_root_get,
};
pub use projects_list::all_projects_list;
pub use user::{handle_settings, settings_page, user_page};

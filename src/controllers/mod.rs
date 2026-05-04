mod auth;
mod content_page;
mod events;
mod landing;
mod meta;
mod not_found;
mod project;
mod user;

pub use auth::{handle_login, handle_registration, login_page, logout, registration_page};
pub use content_page::render_content_page;
pub use events::{global_events, project_events, user_events};
pub use landing::index;
pub use meta::{robots_txt, sitemap_xml};
pub use not_found::{not_found, not_found_get};
pub use project::{
    all_projects_list, git_info_refs, git_upload_pack, project_blob_get, project_branches_get,
    project_commits_get, project_delete_post, project_new_get, project_new_post,
    project_overview_get, project_settings_get, project_settings_post, project_tags_get,
    project_tree_get, project_tree_root_get, talk_comment_post, talk_list_get, talk_new_get,
    talk_new_post, talk_view_get,
};
pub use user::{delete_account, handle_settings, settings_page, user_keys_get, user_page};

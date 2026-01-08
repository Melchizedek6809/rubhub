mod comment;
mod list;
mod new;
mod view;

pub use comment::talk_comment_post;
pub use list::talk_list_get;
pub use new::{talk_new_get, talk_new_post};
pub use view::talk_view_get;

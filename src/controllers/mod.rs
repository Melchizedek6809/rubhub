mod not_found;
mod auth;
mod contact;
mod landing;
mod user;

pub mod project;

pub use user::{settings_page, handle_settings, user_page};
pub use contact::contact;
pub use landing::index;
pub use not_found::not_found;
pub use auth::{handle_login, login_page, logout, handle_registration, registration_page};

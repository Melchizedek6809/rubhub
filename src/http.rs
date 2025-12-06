use axum::{Router, http::StatusCode, response::Html, routing::get};
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
use tower_http::services::ServeDir;

use crate::{app, pages, state::GlobalState};

pub async fn start_http_server(state: GlobalState) -> anyhow::Result<()> {
    let public_assets_dir = state.config.asset_root.join("public");
    let bind_addr = state.config.http_bind_addr;

    // build our application with a single route
    let app = Router::new()
        .route("/", get(pages::landing::index))
        .route("/contact", get(|| async { Html(app::contact().await) }))
        .route(
            "/login",
            get(pages::auth::login_page).post(pages::auth::handle_login),
        )
        .route("/logout", get(pages::auth::logout))
        .route(
            "/settings",
            get(pages::user::settings_page).post(pages::user::handle_settings),
        )
        .route(
            "/projects/new",
            get(pages::project::new_project_page).post(pages::project::handle_new_project),
        )
        .route("/{username}", get(pages::project::projects_page))
        .route("/{username}/{slug}", get(pages::project::project_page))
        .route(
            "/{username}/{slug}/settings",
            get(pages::project::project_settings_page)
                .post(pages::project::handle_project_settings),
        )
        .nest_service(
            "/public",
            ServiceBuilder::new().service(ServeDir::new("public")),
        )
        .nest_service(
            "/dist",
            ServiceBuilder::new().service(ServeDir::new("dist")),
        )
        .nest_service(
            "/assets",
            ServiceBuilder::new().service(ServeDir::new(public_assets_dir)),
        )
        .fallback(|| async { (StatusCode::NOT_FOUND, Html(app::not_found().await)) })
        .layer(CookieManagerLayer::new())
        .with_state(state.clone());

    let socket = tokio::net::TcpSocket::new_v4()?;
    socket.set_reuseaddr(true)?;

    // Enable reuseport on Linux, that way we can run multiple replicas
    #[cfg(target_os = "linux")]
    socket.set_reuseport(true)?;

    socket.bind(bind_addr)?;

    let listener = socket.listen(1024)?;

    println!("rubhub ready on {bind_addr}");
    axum::serve(listener, app).await?;

    Ok(())
}

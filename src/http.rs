use axum::{
    Router,
    extract::Path,
    http::{StatusCode, header},
    response::{Html, IntoResponse},
    routing::get,
};
use rust_embed::Embed;
use tower_cookies::CookieManagerLayer;

use crate::{GlobalState, app, pages};

#[derive(Embed)]
#[folder = "dist/"]
struct EmbeddedDist;

pub async fn start_http_server(state: GlobalState) -> anyhow::Result<()> {
    let bind_addr = state.config.http_bind_addr;
    let process_start = state.process_start;

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
        .route("/{username}", get(pages::project::project_list_page))
        .route("/{username}/{slug}", get(pages::project::project_page))
        .route(
            "/{username}/{slug}/tree/{branch}",
            get(pages::project::project_page_tree),
        )
        .route(
            "/{username}/{slug}/commits/{branch}",
            get(pages::project::project_page_commits),
        )
        .route(
            "/{username}/{slug}/settings",
            get(pages::project::project_settings_page)
                .post(pages::project::handle_project_settings),
        )
        .route(
            "/dist/{*path}",
            get(|Path(path): Path<String>| async move {
                match EmbeddedDist::get(path.as_str()) {
                    Some(asset) => {
                        let mime = mime_guess::from_path(&path).first_or_octet_stream();
                        (
                            [(header::CONTENT_TYPE, mime.as_ref())],
                            asset.data.into_owned(),
                        )
                            .into_response()
                    }
                    None => (StatusCode::NOT_FOUND, Html(app::not_found().await)).into_response(),
                }
            }),
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

    println!(
        "[{:?}] - RubHub HTTP ready on {bind_addr}",
        process_start.elapsed()
    );
    axum::serve(listener, app).await?;

    Ok(())
}

use axum::{
    Router,
    extract::Path,
    http::header,
    response::IntoResponse,
    routing::get,
    serve::Serve,
};
use rust_embed::Embed;
use tokio::net::TcpListener;
use tower_cookies::CookieManagerLayer;

use crate::{GlobalState, controllers};

#[derive(Embed)]
#[folder = "dist/"]
struct EmbeddedDist;

pub async fn http_server(
    state: GlobalState,
) -> anyhow::Result<Serve<TcpListener, Router<()>, Router<()>>> {
    let bind_addr = state.config.http_bind_addr;
    let process_start = state.process_start;

    // build our application with a single route
    let app = Router::new()
        .route("/", get(controllers::index))
        .route(
            "/contact",
            get(get(controllers::contact)),
        )
        .route(
            "/login",
            get(controllers::login_page).post(controllers::handle_login),
        )
        .route(
            "/registration",
            get(controllers::registration_page).post(controllers::handle_registration),
        )
        .route("/logout", get(controllers::logout))
        .route(
            "/settings",
            get(controllers::settings_page).post(controllers::handle_settings),
        )
        .route(
            "/projects/new",
            get(controllers::project::new_project_page)
                .post(controllers::project::handle_new_project),
        )
        .route("/{username}", get(controllers::user_page))
        .route(
            "/{username}/{slug}",
            get(controllers::project::project_page),
        )
        .route(
            "/{username}/{slug}/branches",
            get(controllers::project::project_page_branches),
        )
        .route(
            "/{username}/{slug}/tags",
            get(controllers::project::project_page_tags),
        )
        .route(
            "/{username}/{slug}/tree/{branch}",
            get(controllers::project::project_page_tree),
        )
        .route(
            "/{username}/{slug}/log/{branch}",
            get(controllers::project::project_page_commits),
        )
        .route(
            "/{username}/{slug}/settings",
            get(controllers::project::project_settings_page)
                .post(controllers::project::handle_project_settings),
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
                    None => controllers::not_found()
                        .await
                        .into_response(),
                }
            }),
        )
        .fallback(controllers::not_found)
        .layer(CookieManagerLayer::new())
        .with_state(state.clone());

    let socket = tokio::net::TcpSocket::new_v4()?;
    socket.set_reuseaddr(true)?;

    // Enable reuseport on Linux, that way we can run multiple replicas
    #[cfg(target_os = "linux")]
    socket.set_reuseport(state.config.reuse_port)?;

    socket.bind(bind_addr)?;

    let listener = socket.listen(1024)?;

    println!(
        "[{:?}] - RubHub HTTP ready on {bind_addr}",
        process_start.elapsed()
    );

    Ok(axum::serve(listener, app))
}

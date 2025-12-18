use axum::{
    Router,
    extract::{Path, State},
    http::header,
    response::IntoResponse,
    routing::get,
    serve::Serve,
};
use rust_embed::Embed;
use tokio::net::TcpListener;
use tower_cookies::{CookieManagerLayer, Cookies};

use crate::{GlobalState, controllers, services::session};

#[derive(Embed)]
#[folder = "dist/"]
struct EmbeddedDist;

pub async fn http_server(
    state: GlobalState,
    listener: TcpListener,
) -> anyhow::Result<Serve<TcpListener, Router<()>, Router<()>>> {
    let bind_addr = listener.local_addr()?;
    let process_start = state.process_start;

    // build our application with a single route
    let mut app = Router::new()
        .route("/", get(controllers::index))
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
                get(controllers::project_new_get).post(controllers::project_new_post),
            );

    // Dynamically register content page routes
    for page in &state.config.content_pages {
        let page_clone = page.clone();
        let route_path = page.url_path();

        app = app.route(
            &route_path,
            get(move |state: State<GlobalState>, cookies: Cookies| {
                let page = page_clone.clone();
                async move { controllers::render_content_page(state, cookies, page).await }
            }),
        );
    }

    let app = app
            .route("/{username}", get(controllers::user_page))
            .route("/{username}/{slug}", get(controllers::project_overview_get))
            .route(
                "/{username}/{slug}/branches",
                get(controllers::project_branches_get),
            )
            .route(
                "/{username}/{slug}/tags",
                get(controllers::project_tags_get),
            )
            .route(
                "/{username}/{slug}/tree/{branch}",
                get(controllers::project_overview_tree_get),
            )
            .route(
                "/{username}/{slug}/log/{branch}",
                get(controllers::project_commits_get),
            )
            .route(
                "/{username}/{slug}/settings",
                get(controllers::project_settings_get).post(controllers::project_settings_post),
            )
            .route(
                "/{username}/{slug}/delete",
                axum::routing::post(controllers::project_delete_post),
            )
            // Git HTTP protocol endpoints (must come after specific routes)
            .route(
                "/{username}/{slug}/info/refs",
                get(controllers::git_info_refs),
            )
            .route(
                "/{username}/{slug}/git-upload-pack",
                axum::routing::post(controllers::git_upload_pack),
            )
            .route(
                "/dist/{*path}",
                get(
                    |Path(path): Path<String>,
                     State(state): State<GlobalState>,
                     cookies: Cookies| async move {
                        match EmbeddedDist::get(path.as_str()) {
                            Some(asset) => {
                                let mime = mime_guess::from_path(&path).first_or_octet_stream();
                                (
                                    [(header::CONTENT_TYPE, mime.as_ref())],
                                    asset.data.into_owned(),
                                )
                                    .into_response()
                            }
                            None => {
                                let logged_in_user =
                                    session::current_user(&state, &cookies).await.ok();
                                controllers::not_found(logged_in_user, vec![])
                            }
                        }
                    },
                ),
            )
            .fallback(controllers::not_found_get)
            .layer(CookieManagerLayer::new())
            .with_state(state.clone());

    println!(
        "[{:?}] - RubHub HTTP ready on {bind_addr}",
        process_start.elapsed()
    );

    Ok(axum::serve(listener, app))
}

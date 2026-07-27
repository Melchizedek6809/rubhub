use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderName, HeaderValue, Response, header},
    response::IntoResponse,
    routing::{get, post},
};
use rust_embed::{Embed, RustEmbed};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tower_cookies::{CookieManagerLayer, Cookies};
use tower_governor::{
    GovernorLayer, governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor,
};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::{GlobalState, controllers, controllers::context::PageContext};

#[derive(Embed)]
#[folder = "dist/"]
struct EmbeddedDist;

#[derive(Embed)]
#[folder = "public/"]
struct EmbeddedPublic;

fn embedded_asset_response<E: RustEmbed>(path: &str) -> Option<Response<Body>> {
    E::get(path).map(|asset| {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        (
            [(header::CONTENT_TYPE, mime.as_ref())],
            asset.data.into_owned(),
        )
            .into_response()
    })
}

async fn embedded_asset_or_not_found<E: RustEmbed>(
    path: &str,
    state: &GlobalState,
    cookies: &Cookies,
) -> Response<Body> {
    if let Some(response) = embedded_asset_response::<E>(path) {
        response
    } else {
        PageContext::load(state, cookies).await.not_found()
    }
}

pub async fn http_server(
    state: GlobalState,
    listener: TcpListener,
) -> anyhow::Result<impl std::future::IntoFuture<Output = Result<(), std::io::Error>>> {
    let bind_addr = listener.local_addr()?;
    let process_start = state.process_start;

    // Rate limiting for auth POST routes: 10 requests per 60 seconds per IP
    let auth_rate_limit = GovernorConfigBuilder::default()
        .key_extractor(SmartIpKeyExtractor)
        .period(Duration::from_secs(60))
        .burst_size(10)
        .finish()
        .expect("Failed to build rate limiter config");

    let auth_post_routes = Router::new()
        .route("/login", axum::routing::post(controllers::handle_login))
        .layer(GovernorLayer::new(auth_rate_limit));

    // build our application with a single route
    let mut app = Router::new()
        .route("/", get(controllers::index))
        .route(
            "/favicon.ico",
            get(
                |State(state): State<GlobalState>, cookies: Cookies| async move {
                    embedded_asset_or_not_found::<EmbeddedPublic>("favicon.ico", &state, &cookies)
                        .await
                },
            ),
        )
        .route(
            "/favicon.png",
            get(
                |State(state): State<GlobalState>, cookies: Cookies| async move {
                    embedded_asset_or_not_found::<EmbeddedPublic>("favicon.png", &state, &cookies)
                        .await
                },
            ),
        )
        .route("/robots.txt", get(controllers::robots_txt))
        .route("/sitemap.xml", get(controllers::sitemap_xml))
        .route("/login", get(controllers::login_page))
        .merge(auth_post_routes)
        .route("/logout", post(controllers::logout))
        .route(
            "/settings",
            get(controllers::settings_page).post(controllers::handle_settings),
        )
        .route(
            "/settings/delete/{delete_token}",
            post(controllers::delete_account),
        )
        .route(
            "/projects/new",
            get(controllers::project_new_get).post(controllers::project_new_post),
        )
        .route("/projects", get(controllers::all_projects_list))
        // SSE event stream (global)
        .route("/.events", get(controllers::global_events));

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

    let app =
        app
            // SSE event streams (must come before catch-all patterns)
            .route("/{username}/.events", get(controllers::user_events))
            .route("/{username}/{slug}/.events", get(controllers::project_events))
            .route("/{username}", get(controllers::user_page))
            .route("/{username}/.keys", get(controllers::user_keys_get))
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
                "/{username}/{slug}/tree/{ref}/{*path}",
                get(controllers::project_tree_get),
            )
            .route(
                "/{username}/{slug}/tree/{ref}",
                get(controllers::project_tree_root_get),
            )
            .route(
                "/{username}/{slug}/blob/{ref}/{*path}",
                get(controllers::project_blob_get),
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
            // Talk routes
            .route(
                "/{username}/{slug}/talk",
                get(controllers::talk_list_get),
            )
            .route(
                "/{username}/{slug}/talk/new",
                get(controllers::talk_new_get).post(controllers::talk_new_post),
            )
            .route(
                "/{username}/{slug}/talk/{issue_dir}",
                get(controllers::talk_view_get),
            )
            .route(
                "/{username}/{slug}/talk/{issue_dir}/comment",
                axum::routing::post(controllers::talk_comment_post),
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
                        embedded_asset_or_not_found::<EmbeddedDist>(&path, &state, &cookies).await
                    },
                ),
            )
        .route(
                "/public/{*path}",
                get(
                    |Path(path): Path<String>,
                     State(state): State<GlobalState>,
                     cookies: Cookies| async move {
                        embedded_asset_or_not_found::<EmbeddedPublic>(&path, &state, &cookies).await
                    },
                ),
            )
            .fallback(controllers::not_found_get)
        .layer(CookieManagerLayer::new())
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static("default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' blob: data: https:; connect-src 'self'; form-action 'self'; upgrade-insecure-requests; block-all-mixed-content;")
        ))
        .with_state(state.clone());

    println!(
        "[{:?}] - RubHub HTTP ready on {bind_addr}",
        process_start.elapsed()
    );

    Ok(axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    ))
}

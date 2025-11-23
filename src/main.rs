use axum::{Router, response::Html, routing::get};
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
use tower_http::services::ServeDir;

mod api;
mod app;
mod state;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Starting jam2nite");
    #[cfg(debug_assertions)]
    dotenvy::dotenv()?;

    println!("Creating new state");
    let state = state::GlobalState::new().await?;

    let public_assets_dir = state.config.asset_root.join("public");
    let bind_addr = state.config.bind_addr;

    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { Html(app::index().await) }))
        .nest("/api", api::router())
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
        .layer(CookieManagerLayer::new())
        .with_state(state);

    let socket = tokio::net::TcpSocket::new_v4()?;

    socket.set_reuseaddr(true)?;
    assert!(socket.reuseaddr().unwrap());

    #[cfg(target_os = "linux")]
    {
        println!("Setting SO_REUSEPORT since we're running on Linux");
        socket.set_reuseport(true)?;
        assert!(socket.reuseport().unwrap());
    }

    socket.bind(bind_addr)?;

    let listener = socket.listen(1024)?;

    println!("jam2nite listening on {bind_addr}");
    axum::serve(listener, app).await?;

    Ok(())
}

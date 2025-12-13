use tokio::runtime::Builder;

mod controllers;
mod extractors;
mod http;
mod models;
mod services;
mod ssh;
mod state;
mod views;

pub use models::{AccessType, Project, ProjectSummary, User};
pub use state::{AppConfig, GlobalState};

pub async fn run<T: Future>(state: GlobalState, kill: T) {
    println!("[{:?}] - RubHub started", state.process_start.elapsed());

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    let http_server = http::http_server(state.clone())
        .await
        .expect("Couldn't start http_server");
    let ssh_server = ssh::ssh_server(state.clone())
        .await
        .expect("Couldn't start ssh_server");

    println!("[{:?}] - RubHub ready", state.process_start.elapsed());

    tokio::select! {
        http_res = http_server => {
            eprintln!("HTTP server stopped: {:?}", http_res);
        }
        ssh_res = ssh_server => {
            eprintln!("SSH server stopped: {:?}", ssh_res);
        }
        signal_res = tokio::signal::ctrl_c() => {
            eprintln!("Received Signal: {:?}", signal_res);
        }
        term_res = terminate => {
            eprintln!("Received Terminate Signal: {:?}", term_res);
        }
        _ = kill => {
            eprintln!("Received Kill!");
        }
    }
}

pub fn run_single_thread(state: GlobalState) {
    // We're using the single threaded runtime, mainly because
    // we can just run multiple processes, that way we also
    // utilize multiple cores but also gain more resiliency
    // since a panic will only bring down 1 application server
    // and hopefully not error out too many in-flight requests.
    //
    // Additionally it makes deadlock detection much simpler,
    // that way we can just observe the server from the outside
    // and if it doesn't respond to a heartbeat/healthcheck quick
    // enough we'll just restart it.
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Couldn't start tokio runtime");

    runtime.block_on(async {
        let kill = std::future::pending::<()>();
        run(state, kill).await
    });
}

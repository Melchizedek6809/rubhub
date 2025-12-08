use tokio::runtime::Builder;

mod app;
mod entities;
mod http;
mod pages;
mod services;
mod ssh;
mod state;

fn main() {
    let start = std::time::Instant::now();

    #[cfg(debug_assertions)]
    if dotenvy::dotenv().is_err() {
        println!("No .env found, using dev defaults");
    }

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
        let state = state::GlobalState::new(start)
            .await
            .expect("Couldn't create GlobalState");

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            http_res = http::start_http_server(state.clone()) => {
                eprintln!("HTTP server stopped: {:?}", http_res);
            }
            ssh_res = ssh::start_ssh_server(state.clone()) => {
                eprintln!("SSH server stopped: {:?}", ssh_res);
            }
            signal_res = tokio::signal::ctrl_c() => {
                eprintln!("Received Signal: {:?}", signal_res);
            }
            term_res = terminate => {
                eprintln!("Received Terminate Signal: {:?}", term_res);
            }
        }
    });
}

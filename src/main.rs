use tokio::runtime::Builder;

mod app;
mod entities;
mod http;
mod pages;
mod services;
mod ssh;
mod state;

fn main() {
    #[cfg(debug_assertions)]
    if dotenvy::dotenv().is_err() {
        println!("No .env found, using dev defaults");
    }

    let runtime = Builder::new_multi_thread()
        .worker_threads(2) // <-- your number here
        .max_blocking_threads(1024)
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async {
        let state = state::GlobalState::new()
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

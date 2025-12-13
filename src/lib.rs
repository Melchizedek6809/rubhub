use std::time::Duration;

use tokio::runtime::Builder;
use sd_notify::notify;
use sd_notify::NotifyState;

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
use tokio::time::interval;


fn systemd_integration() {
    // Tell systemd we are ready (no-op if not under systemd)
    let _ = notify(false, &[NotifyState::Ready]);

    // WATCHDOG_USEC is only set if watchdog is enabled *and* systemd manages us
    let watchdog_usec = match std::env::var("WATCHDOG_USEC") {
        Ok(v) => v.parse::<u64>().ok(),
        Err(_) => None,
    };

    let watchdog_usec = match watchdog_usec {
        Some(v) if v > 0 => v,
        _ => return, // no watchdog → nothing to do
    };

    // systemd recommends pinging at least every WatchdogSec / 2
    // we use /3 for extra margin
    let interval_duration =
        Duration::from_micros(watchdog_usec / 3);

    tokio::spawn(async move {
        let mut ticker = interval(interval_duration);

        loop {
            ticker.tick().await;
            let _ = notify(false, &[NotifyState::Watchdog]);
        }
    });
}

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

    systemd_integration();

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

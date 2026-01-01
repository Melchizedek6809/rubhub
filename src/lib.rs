use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use sd_notify::NotifyState;
use sd_notify::notify;
use tokio::net::{TcpListener, TcpSocket};
use tokio::runtime::Builder;

mod controllers;
mod extractors;
mod http;
mod models;
mod services;
mod ssh;
mod state;
mod views;

pub use rubhub_auth_store::User;
pub use models::{AccessType, Project, ProjectSummary, UserModel};
pub use state::{AppConfig, GlobalState};
use tokio::time::interval;

/// Create and bind a TCP listener with appropriate socket options
pub fn create_listener(addr: SocketAddr, reuse_port: bool) -> io::Result<TcpListener> {
    let socket = if addr.is_ipv4() {
        TcpSocket::new_v4()?
    } else {
        TcpSocket::new_v6()?
    };

    socket.set_reuseaddr(true)?;

    #[cfg(target_os = "linux")]
    socket.set_reuseport(reuse_port)?;

    socket.bind(addr)?;
    socket.listen(1024)
}

/// Create both HTTP and SSH listeners, returning listeners and their bound addresses
pub fn create_listeners(
    config: &AppConfig,
) -> io::Result<(TcpListener, SocketAddr, TcpListener, SocketAddr)> {
    let http_listener = create_listener(config.http_bind_addr, config.reuse_port)?;
    let http_addr = http_listener.local_addr()?;

    let ssh_listener = create_listener(config.ssh_bind_addr, config.reuse_port)?;
    let ssh_addr = ssh_listener.local_addr()?;

    Ok((http_listener, http_addr, ssh_listener, ssh_addr))
}

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
    let interval_duration = Duration::from_micros(watchdog_usec / 3);

    tokio::spawn(async move {
        let mut ticker = interval(interval_duration);

        loop {
            ticker.tick().await;
            let _ = notify(false, &[NotifyState::Watchdog]);
        }
    });
}

pub async fn run<T: Future>(
    state: GlobalState,
    http_listener: TcpListener,
    ssh_listener: TcpListener,
    kill: T,
) {
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

    let http_server = http::http_server(state.clone(), http_listener)
        .await
        .expect("Couldn't start http_server");
    let ssh_server = ssh::ssh_server(state.clone(), ssh_listener)
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

pub fn run_multi_thread(config: AppConfig, process_start: std::time::Instant) {
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Couldn't start tokio runtime");

    runtime.block_on(async {
        // Create listeners inside the runtime
        let (http_listener, http_addr, ssh_listener, ssh_addr) =
            create_listeners(&config).expect("Failed to create listeners");

        // Update config with actual addresses
        let config = config.update_bound_addresses(http_addr, ssh_addr);

        let state = config
            .build(process_start)
            .expect("Error creating GlobalState from AppConfig");

        let kill = std::future::pending::<()>();
        run(state, http_listener, ssh_listener, kill).await
    });
}

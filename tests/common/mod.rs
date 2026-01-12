#![allow(dead_code, unused_imports)]

mod api;
mod git;
pub mod html;
mod ssh;
mod test_user;

use std::future::Future;

use rubhub::{AppConfig, GlobalState, create_listeners, run};
use tempfile::TempDir;

pub use api::Api;
pub use git::GitHelper;
pub use ssh::{KeyType, TestSshKey};
pub use test_user::{TestUser, assertions};

/// Helper function to run integration tests with a temporary backend
///
/// This function:
/// - Creates a temporary directory for test data
/// - Binds to port 0 for OS-assigned ports (enables parallel tests)
/// - Passes the GlobalState to the test callback
/// - Cleans up after the test completes
pub async fn with_backend<F, Fut>(test: F)
where
    F: FnOnce(GlobalState) -> Fut,
    Fut: Future<Output = ()>,
{
    let process_start = std::time::Instant::now();
    let dir = TempDir::new().expect("Couldn't create TempDir");
    let path = dir.path();
    let dir_root = path.to_str().expect("Couldn't turn TempDir path to_str()");

    println!("TempDir: {dir_root}");

    // Bind to port 0 for OS-assigned ports
    let config = AppConfig::default()
        .set_dir_root(dir_root)
        .set_http_bind_addr("127.0.0.1:0")
        .expect("set_http_bind_addr")
        .set_ssh_bind_addr("127.0.0.1:0")
        .expect("set_ssh_bind_addr");

    // Create listeners and get actual ports
    let (http_listener, http_addr, ssh_listener, ssh_addr) =
        create_listeners(&config).expect("Failed to create listeners");

    println!("HTTP bound to: {}", http_addr);
    println!("SSH bound to: {}", ssh_addr);

    // Update config with actual addresses
    let config = config.update_bound_addresses(http_addr, ssh_addr);
    let state = GlobalState::new(config, process_start).expect("GlobalState");

    // Pass state to test - it can access:
    // - state.config.base_url for HTTP requests
    // - state.config.ssh_public_host for SSH URLs
    // - Any other config as needed
    run(state.clone(), http_listener, ssh_listener, test(state)).await;

    std::fs::remove_dir_all(path).expect("Couldn't clean up TempDir");
}

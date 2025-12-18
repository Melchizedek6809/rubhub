use std::collections::HashMap;
use std::future::Future;

use anyhow::{Result, anyhow};
use reqwest::Client;
use rubhub::{AppConfig, GlobalState, create_listeners, run};
use tempfile::TempDir;

async fn with_backend<F, Fut>(test: F)
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
    let state = config.build(process_start).expect("GlobalState");

    // Pass state to test - it can access:
    // - state.config.base_url for HTTP requests
    // - state.config.ssh_public_host for SSH URLs
    // - Any other config as needed
    run(state.clone(), http_listener, ssh_listener, test(state)).await;

    std::fs::remove_dir_all(path).expect("Couldn't clean up TempDir");
}

async fn response_contains(client: &Client, url: &str, needle: &str) -> Result<()> {
    let body = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    if body.contains(needle) {
        Ok(())
    } else {
        eprintln!("{body}");
        Err(anyhow!(format!("{url} is missing {:?}", needle)))
    }
}

#[tokio::test(flavor = "current_thread")]
async fn basic_workflow() {
    with_backend(|state| async move {
        let base_url = &state.config.base_url;

        let client = reqwest::Client::builder()
            .cookie_store(true)
            .build()
            .expect("Couldn't initialize reqwest client");

        response_contains(&client, &format!("{base_url}/"), "RubHub")
            .await
            .unwrap();

        let mut form = HashMap::new();
        let pw = "12345678901234567890";
        form.insert("username", "t");
        form.insert("email", "test@rubhub.net");
        form.insert("password", pw);

        // First we try to register with a username that's too short
        client
            .post(format!("{base_url}/registration"))
            .form(&form)
            .send()
            .await
            .expect("Registration failed")
            .error_for_status()
            .expect_err("Registration request failed");

        // Now we use the full username
        form.insert("username", "test");
        client
            .post(format!("{base_url}/registration"))
            .form(&form)
            .send()
            .await
            .expect("Registration failed")
            .error_for_status()
            .expect("Registration request failed");

        response_contains(&client, &format!("{base_url}/~test"), "test")
            .await
            .unwrap();

        client
            .get(format!("{base_url}/logout"))
            .send()
            .await
            .expect("Logout failed")
            .error_for_status()
            .expect("Logout status failed");
    })
    .await;
}

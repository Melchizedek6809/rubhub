use std::collections::HashMap;

use anyhow::{Result, anyhow};
use reqwest::Client;
use rubhub::{AppConfig, run};
use tempfile::TempDir;

async fn with_backend<T: Future>(test: T) {
    let process_start = std::time::Instant::now();
    let dir = TempDir::new().expect("Couldn't create TempDir");
    let path = dir.path();
    let dir_root = path.to_str().expect("Couldn't turn TempDir path to_str()");

    println!("TempDir: {dir_root}");

    // ToDo: find a better way to figure out which port to run tests on!
    let config = AppConfig::default()
        .set_dir_root(dir_root)
        .set_http_bind_addr("127.0.0.1:32323")
        .expect("set_http_bind_addr")
        .set_ssh_bind_addr("127.0.0.1:32324")
        .expect("set_ssh_bind_addr");
    let state = config.build(process_start).expect("GlobalState");

    run(state, test).await;

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
    with_backend(async {
        let base = "http://127.0.0.1:32323";

        let client = reqwest::Client::builder()
            .cookie_store(true)
            .build()
            .expect("Couldn't initialize reqwest client");

        response_contains(&client, &format!("{base}/"), "Welcome to RubHub")
            .await
            .unwrap();

        let mut form = HashMap::new();
        let pw = "12345678901234567890";
        form.insert("username", "t");
        form.insert("email", "test@rubhub.net");
        form.insert("password", pw);

        // First we try to register with a username that's too short
        client
            .post(format!("{base}/registration"))
            .form(&form)
            .send()
            .await
            .expect("Registration failed")
            .error_for_status()
            .expect_err("Registration request failed");

        // Now we use the full username
        form.insert("username", "test");
        client
            .post(format!("{base}/registration"))
            .form(&form)
            .send()
            .await
            .expect("Registration failed")
            .error_for_status()
            .expect("Registration request failed");

        response_contains(&client, &format!("{base}/~test"), "test")
            .await
            .unwrap();

        client
            .get(format!("{base}/logout"))
            .send()
            .await
            .expect("Logout failed")
            .error_for_status()
            .expect("Logout status failed");
    })
    .await;
}

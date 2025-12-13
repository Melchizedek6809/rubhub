use anyhow::{Result, anyhow};
use rubhub::{AppConfig, run};

async fn with_backend<T: Future>(test: T) {
    let process_start = std::time::Instant::now();
    // ToDo: find a better way to figure out which port to run tests on!
    let config = AppConfig::default()
        .set_http_bind_addr("127.0.0.1:32323")
        .expect("set_http_bind_addr")
        .set_ssh_bind_addr("127.0.0.1:32324")
        .expect("set_ssh_bind_addr");
    let state = config.build(process_start).expect("GlobalState");

    run(state, test).await;
}

async fn response_contains(url: &str, needle: &str) -> Result<()> {
    let body = reqwest::get(url).await?.text().await?;

    if body.contains(needle) {
        Ok(())
    } else {
        eprintln!("{body}");
        Err(anyhow!(format!("{url} is missing {:?}", needle)))
    }
}

#[tokio::test(flavor = "current_thread")]
async fn start_and_shutdown_cleanly() {
    with_backend(async {
        let base = "http://127.0.0.1:32323";

        response_contains(&format!("{base}/"), "Welcome to RubHub")
            .await
            .unwrap();
        response_contains(&format!("{base}/contact"), "melchizedek6809")
            .await
            .unwrap();
    })
    .await;
}

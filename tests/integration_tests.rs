use rubhub::{AppConfig, run};

#[tokio::test(flavor = "current_thread")]
async fn start_and_shutdown_cleanly() {
    let (tx, rx) = tokio::sync::oneshot::channel();

    let process_start = std::time::Instant::now();
    let config = AppConfig::new().expect("AppConfig");
    let state = config.build(process_start).expect("GlobalState");

    let handle = tokio::spawn(async move {
        run(state, async {
            rx.await.ok();
        })
        .await;
    });

    // give the system time to start
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // trigger shutdown
    let _ = tx.send(());

    // ensure clean exit
    handle.await.unwrap();
}

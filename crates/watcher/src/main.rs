use reqwest::Client;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let client = Client::new();

    loop {
        println!("Trying");
        let mut stream = client
            .get("https://rubhub.net/~ben/rubhub/.events")
            .header("Accept", "text/event-stream")
            .send()
            .await?
            .error_for_status()?;
        println!("Streaming");

        while let Ok(Some(chunk)) = stream.chunk().await {
            let bytes = chunk;
            let text = String::from_utf8_lossy(&bytes);
            print!("{text}");
        }
    };
}

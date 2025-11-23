#[cfg(debug_assertions)]
use tokio::fs;

#[cfg(not(debug_assertions))]
const APP_THEME: &str = include_str!("../dist/app.html");

async fn theme(head: &str, body: &str) -> String {
    #[cfg(not(debug_assertions))]
    let contents = APP_THEME;
    #[cfg(debug_assertions)]
    let contents = fs::read_to_string("dist/app.html").await.unwrap();

    let contents = contents.replace("<!--HEAD-->", head);
    contents.replace("<!--BODY-->", body)
}

pub async fn index() -> String {
    theme("<title>Jam2Nite</title>", "").await
}

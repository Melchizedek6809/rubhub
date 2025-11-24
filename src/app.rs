#[cfg(debug_assertions)]
use tokio::fs;

#[cfg(not(debug_assertions))]
const APP_THEME: &str = include_str!("../dist/app.html");

pub fn extract_html_parts(html: &str) -> (&str, &str) {
    let first_split = html.split_once("</head>").unwrap();
    let head = first_split.0.split_once("<head>").unwrap().1;
    let rest = first_split.1.split_once("<body>").unwrap().1;
    let body = rest.split_once("</body>").unwrap().0;

    (head, body)
}

async fn theme(head: &str, body: &str) -> String {
    #[cfg(not(debug_assertions))]
    let contents = APP_THEME;
    #[cfg(debug_assertions)]
    let contents = fs::read_to_string("dist/app.html").await.unwrap();

    let contents = contents.replace("<!--HEAD-->", head);
    contents.replace("<!--BODY-->", body)
}

#[cfg(not(debug_assertions))]
const CONTENT_INDEX: &str = include_str!("../frontend/app/content.index.html");
pub async fn index() -> String {
    #[cfg(not(debug_assertions))]
    let contents = CONTENT_INDEX;
    #[cfg(debug_assertions)]
    let contents = fs::read_to_string("frontend/app/content.index.html")
        .await
        .unwrap();

    let parts = extract_html_parts(&contents);

    theme(parts.0, parts.1).await
}

#[cfg(not(debug_assertions))]
const CONTENT_404: &str = include_str!("../frontend/app/content.404.html");
pub async fn not_found() -> String {
    #[cfg(not(debug_assertions))]
    let contents = CONTENT_404;
    #[cfg(debug_assertions)]
    let contents = fs::read_to_string("frontend/app/content.404.html")
        .await
        .unwrap();

    let parts = extract_html_parts(&contents);

    theme(parts.0, parts.1).await
}

#[cfg(test)]
mod tests {
    use super::extract_html_parts;

    #[test]
    fn extract_html_parts_returns_head_and_body() {
        let html = "<html><head><title>Test</title></head><body><h1>Hi</h1></body></html>";
        let (head, body) = extract_html_parts(html);

        assert_eq!(head, "<title>Test</title>");
        assert_eq!(body, "<h1>Hi</h1>");
    }

    #[test]
    fn extract_html_parts_handles_whitespace() {
        let html =
            "<html><head>\n<meta charset=\"utf-8\">\n</head><body>\n<p>Test</p>\n</body></html>";
        let (head, body) = extract_html_parts(html);

        assert_eq!(head, "\n<meta charset=\"utf-8\">\n");
        assert_eq!(body, "\n<p>Test</p>\n");
    }
}

#[cfg(debug_assertions)]
use tokio::fs;

#[cfg(not(debug_assertions))]
const APP_THEME: &str = include_str!("../../dist/app.html");

pub fn extract_html_parts(html: &str) -> (&str, &str) {
    let first_split = html
        .split_once("</head>")
        .expect("extract_html_parts first_split");
    let head = first_split
        .0
        .split_once("<head>")
        .expect("extract_html_parts head")
        .1;
    let rest = first_split
        .1
        .split_once("<body>")
        .expect("extract_html_parts rest")
        .1;
    let body = rest
        .split_once("</body>")
        .expect("extract_html_parts body")
        .0;

    (head, body)
}

pub async fn theme_render(head: &str, body: &str) -> String {
    #[cfg(not(debug_assertions))]
    let contents = APP_THEME;
    #[cfg(debug_assertions)]
    let contents = fs::read_to_string("dist/app.html").await.unwrap();

    let contents = contents.replace("<!--HEAD-->", head);
    contents.replace("<!--BODY-->", body)
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

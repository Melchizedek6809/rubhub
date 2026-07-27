mod common;

use common::{Api, with_backend};
use reqwest::StatusCode;

#[tokio::test(flavor = "current_thread")]
async fn robots_txt_is_served_from_root() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);

        let body = api.get_text("/robots.txt").await.unwrap();

        assert!(body.contains("User-agent: *"));
        assert!(body.contains("Allow: /"));
        assert!(body.contains("Disallow: /~*/branches"));
        assert!(body.contains("Disallow: /~*/tags"));
        assert!(body.contains("Disallow: /~*/log/"));
        assert!(body.contains("Disallow: /~*/tree/"));
        assert!(body.contains("Disallow: /~*/blob/"));
        assert!(body.contains(&format!("Sitemap: {}/sitemap.xml", state.config.base_url)));
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn utility_pages_are_marked_noindex() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);

        let login = api.get_text("/login").await.unwrap();
        assert!(login.contains(r#"<meta name="robots" content="noindex,follow">"#));

        let missing = api.get_raw("/dist/missing").await.unwrap();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        let missing_body = missing.text().await.unwrap();
        assert!(missing_body.contains(r#"<meta name="robots" content="noindex,follow">"#));
    })
    .await;
}

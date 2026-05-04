mod common;

use common::{Api, with_backend};
use reqwest::StatusCode;

fn assert_themed_404(body: &str) {
    assert!(body.contains("<h1>404</h1>"));
    assert!(body.contains("Page not found"));
    assert!(body.contains("Browse"));
}

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
async fn sitemap_lists_public_metadata_urls_only() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);

        api.register("alice", "alice@example.com", "alicepassword123")
            .await
            .unwrap();
        api.create_project_with_access("Public Project", "Public", "read")
            .await
            .unwrap();
        api.create_project_with_access("Private Project", "Private", "none")
            .await
            .unwrap();

        let body = api.get_text("/sitemap.xml").await.unwrap();

        assert!(body.contains("<urlset"));
        assert!(body.contains(&format!("<loc>{}/</loc>", state.config.base_url)));
        assert!(body.contains(&format!("<loc>{}/projects</loc>", state.config.base_url)));
        assert!(body.contains(&format!("<loc>{}/~alice</loc>", state.config.base_url)));
        assert!(body.contains(&format!(
            "<loc>{}/~alice/public-project</loc>",
            state.config.base_url
        )));
        assert!(!body.contains("private-project"));
        assert!(!body.contains("/tree/"));
        assert!(!body.contains("/blob/"));
        assert!(!body.contains("/settings"));
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn public_project_page_renders_machine_readable_metadata() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);
        let long_description = "Long description ".repeat(300);

        api.register("alice", "alice@example.com", "alicepassword123")
            .await
            .unwrap();
        api.create_project("Public Project", "ignored during create")
            .await
            .unwrap();
        api.update_project_settings(
            "alice",
            "public-project",
            "Public Project",
            &long_description,
            "read",
            "main",
            "",
        )
        .await
        .unwrap();

        let body = api.get_text("/~alice/public-project").await.unwrap();
        let expected_description = format!("{}...", &"Long description ".repeat(10)[..157]);

        assert!(body.contains("<title>alice/Public Project - RubHub</title>"));
        assert!(body.contains(&format!(
            r#"<meta name="description" content="{expected_description}">"#
        )));
        assert!(!body.contains(&long_description));
        assert!(body.contains(&format!(
            r#"<link rel="canonical" href="{}/~alice/public-project">"#,
            state.config.base_url
        )));
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn utility_pages_are_marked_noindex() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);

        let login = api.get_text("/login").await.unwrap();
        assert!(login.contains(r#"<meta name="robots" content="noindex,follow">"#));

        let registration = api.get_text("/registration").await.unwrap();
        assert!(registration.contains(r#"<meta name="robots" content="noindex,follow">"#));

        let missing = api.get_raw("/dist/missing").await.unwrap();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        let missing_body = missing.text().await.unwrap();
        assert!(missing_body.contains(r#"<meta name="robots" content="noindex,follow">"#));
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn anonymous_profile_does_not_show_private_projects() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);
        let anonymous = Api::new(&state.config.base_url);

        api.register("alice", "alice@example.com", "alicepassword123")
            .await
            .unwrap();
        api.create_project_with_access("Public Project", "Public", "read")
            .await
            .unwrap();
        api.create_project_with_access("Private Project", "Private", "none")
            .await
            .unwrap();

        let body = anonymous.get_text("/~alice").await.unwrap();

        assert!(body.contains("Public Project"));
        assert!(!body.contains("Private Project"));
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn missing_and_hidden_resources_render_themed_404() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);
        let anonymous = Api::new(&state.config.base_url);

        let missing_user = api.get_raw("/~missing-user").await.unwrap();
        assert_eq!(missing_user.status(), StatusCode::NOT_FOUND);
        assert_themed_404(&missing_user.text().await.unwrap());

        api.register("alice", "alice@example.com", "alicepassword123")
            .await
            .unwrap();
        api.create_project("Visible Project", "A project in the sidebar")
            .await
            .unwrap();
        api.create_project_with_access("Private Project", "Secret", "none")
            .await
            .unwrap();

        let missing_project = api.get_raw("/~alice/missing-project").await.unwrap();
        assert_eq!(missing_project.status(), StatusCode::NOT_FOUND);
        let missing_project_body = missing_project.text().await.unwrap();
        assert_themed_404(&missing_project_body);
        assert!(missing_project_body.contains("Visible Project"));

        let private_project = anonymous.get_raw("/~alice/private-project").await.unwrap();
        assert_eq!(private_project.status(), StatusCode::NOT_FOUND);
        assert_themed_404(&private_project.text().await.unwrap());
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn auth_workflow() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);

        api.assert_contains("/", "RubHub").await.unwrap();

        api.register("t", "test@rubhub.net", "12345678901234567890")
            .await
            .expect_err("Registration should fail for short username");

        api.register("test", "", "12345678901234567890")
            .await
            .expect_err("Registration should fail for missing emails");

        api.register("test", "asdqwezxc", "12345678901234567890")
            .await
            .expect_err("Registration should fail for invalid emails");

        api.register("test", "test@rubhub.net", "123")
            .await
            .expect_err("Registration should fail for short passwords");

        api.register("test", "test@rubhub.net", "12345678901234567890")
            .await
            .unwrap();

        api.assert_contains("/~test", "Settings").await.unwrap();

        api.logout().await.unwrap();

        assert!(state.auth.get_sessions_for_user("test").is_empty());
        api.assert_contains("/~test", "Settings").await.unwrap_err();
        api.login("test", "zxc").await.unwrap_err();

        api.login("test", "12345678901234567890").await.unwrap();

        api.assert_contains("/~test", "Settings").await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_create_project() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);

        api.register("testuser", "test@example.com", "password123456789")
            .await
            .unwrap();

        api.create_project("Test Project", "A test project")
            .await
            .unwrap();

        api.assert_contains("/~testuser/test-project", "Test Project")
            .await
            .unwrap();
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_project_visibility() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);

        api.register("alice", "alice@example.com", "alicepassword123")
            .await
            .unwrap();

        api.create_project("Alice Project", "Alice's project")
            .await
            .unwrap();

        api.logout().await.unwrap();

        // Verify project is still accessible when logged out (public by default)
        api.assert_contains("/~alice/alice-project", "Alice Project")
            .await
            .unwrap();
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_delete_account_removes_user_repos_and_allows_reuse() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);
        let second_session = Api::new(&state.config.base_url);

        api.register("alice", "alice@example.com", "alicepassword123")
            .await
            .unwrap();
        api.create_project("Alice Project", "Alice's project")
            .await
            .unwrap();
        second_session
            .login("alice", "alicepassword123")
            .await
            .unwrap();

        let wrong_token_response = api
            .delete_account_raw("wrong-token", "alice")
            .await
            .unwrap();
        assert_eq!(wrong_token_response.status(), StatusCode::BAD_REQUEST);
        assert!(state.auth.get_user("alice").is_some());
        assert!(state.config.git_root.join("alice").exists());

        api.delete_account("alice").await.unwrap();

        assert!(state.auth.get_user("alice").is_none());
        assert!(
            state
                .auth
                .get_user_slug_by_email("alice@example.com")
                .is_none()
        );
        assert!(state.auth.get_projects_for_owner("alice").is_empty());
        assert!(state.auth.get_sessions_for_user("alice").is_empty());
        assert!(!state.config.git_root.join("alice").exists());

        let profile_response = api.get_raw("/~alice").await.unwrap();
        assert_eq!(profile_response.status(), StatusCode::NOT_FOUND);

        let project_response = api.get_raw("/~alice/alice-project").await.unwrap();
        assert_eq!(project_response.status(), StatusCode::NOT_FOUND);

        api.register("alice", "alice@example.com", "alicepassword123")
            .await
            .unwrap();
    })
    .await;
}

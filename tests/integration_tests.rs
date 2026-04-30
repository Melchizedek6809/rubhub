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

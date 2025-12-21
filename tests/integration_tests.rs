mod common;

use common::{Api, with_backend};

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

/// Test that CSRF protection rejects requests without valid tokens.
#[tokio::test(flavor = "current_thread")]
async fn test_csrf_protection() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);

        // First register a user normally (with CSRF token)
        api.register("alice", "alice@test.com", "password123456789")
            .await
            .unwrap();

        // Try to create a project without CSRF token - should fail
        let response = api
            .post_without_csrf(
                "/projects/new",
                &[("name", "Hacked Project"), ("description", "No CSRF")],
            )
            .await
            .unwrap();

        // Should return 403 Forbidden for missing CSRF token
        assert_eq!(
            response.status().as_u16(),
            403,
            "Request without CSRF token should be rejected with 403"
        );

        // Try login without CSRF token - should also fail
        let login_response = api
            .post_without_csrf(
                "/login",
                &[("username", "alice"), ("password", "password123456789")],
            )
            .await
            .unwrap();

        assert_eq!(
            login_response.status().as_u16(),
            403,
            "Login without CSRF token should be rejected with 403"
        );

        // Try registration without CSRF token - should fail
        let reg_response = api
            .post_without_csrf(
                "/registration",
                &[
                    ("username", "hacker"),
                    ("email", "hacker@evil.com"),
                    ("password", "hackerpassword123"),
                ],
            )
            .await
            .unwrap();

        assert_eq!(
            reg_response.status().as_u16(),
            403,
            "Registration without CSRF token should be rejected with 403"
        );
    })
    .await;
}

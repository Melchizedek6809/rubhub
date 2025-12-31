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

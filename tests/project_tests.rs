mod common;

use std::collections::HashMap;

use common::{response_contains, test_client, with_backend};

#[tokio::test(flavor = "current_thread")]
async fn test_create_project() {
    with_backend(|state| async move {
        let base_url = &state.config.base_url;
        let client = test_client();

        // First, register a user
        let mut form = HashMap::new();
        form.insert("username", "testuser");
        form.insert("email", "test@example.com");
        form.insert("password", "password123456789");

        client
            .post(format!("{base_url}/registration"))
            .form(&form)
            .send()
            .await
            .expect("Registration failed")
            .error_for_status()
            .expect("Registration request failed");

        // Navigate to new project page
        response_contains(&client, &format!("{base_url}/projects/new"), "Create")
            .await
            .unwrap();

        // Create a new project
        let mut project_form = HashMap::new();
        project_form.insert("name", "Test Project");
        project_form.insert("description", "A test project");

        client
            .post(format!("{base_url}/projects/new"))
            .form(&project_form)
            .send()
            .await
            .expect("Project creation failed")
            .error_for_status()
            .expect("Project creation request failed");

        // Verify project page loads
        response_contains(
            &client,
            &format!("{base_url}/~testuser/test-project"),
            "Test Project",
        )
        .await
        .unwrap();
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_project_visibility() {
    with_backend(|state| async move {
        let base_url = &state.config.base_url;
        let client = test_client();

        // Register and create a project
        let mut form = HashMap::new();
        form.insert("username", "alice");
        form.insert("email", "alice@example.com");
        form.insert("password", "alicepassword123");

        client
            .post(format!("{base_url}/registration"))
            .form(&form)
            .send()
            .await
            .expect("Registration failed")
            .error_for_status()
            .expect("Registration request failed");

        // Create project
        let mut project_form = HashMap::new();
        project_form.insert("name", "Alice Project");
        project_form.insert("description", "Alice's project");

        client
            .post(format!("{base_url}/projects/new"))
            .form(&project_form)
            .send()
            .await
            .expect("Project creation failed");

        // Logout
        client
            .get(format!("{base_url}/logout"))
            .send()
            .await
            .expect("Logout failed");

        // Verify project is still accessible when logged out (public by default)
        response_contains(
            &client,
            &format!("{base_url}/~alice/alice-project"),
            "Alice Project",
        )
        .await
        .unwrap();
    })
    .await;
}

//! Tests for the .profile repository feature.
//!
//! The .profile repo stores user metadata (README.md with frontmatter) and SSH keys
//! (authorized_keys file) in a git repository at data/git/{username}/.profile/

mod common;

use common::{Api, TestSshKey, with_backend};

/// Test that registering a user creates a .profile repository.
#[tokio::test(flavor = "current_thread")]
async fn test_registration_creates_profile_repo() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);

        api.register("alice", "alice@test.com", "password123456789")
            .await
            .unwrap();

        // Check that .profile repo was created
        let profile_path = state.config.git_root.join("alice").join(".profile");
        assert!(
            profile_path.exists(),
            ".profile repo should exist at {:?}",
            profile_path
        );

        // Check it's a bare git repo (has HEAD file)
        assert!(
            profile_path.join("HEAD").exists(),
            ".profile should be a bare git repo"
        );
    })
    .await;
}

/// Test that .profile repo contains README.md with user metadata.
#[tokio::test(flavor = "current_thread")]
async fn test_profile_readme_created() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);

        api.register("bob", "bob@example.com", "password123456789")
            .await
            .unwrap();

        // Use git to read the README.md from the .profile repo
        let profile_path = state.config.git_root.join("bob").join(".profile");
        let output = tokio::process::Command::new("git")
            .args(["show", "main:README.md"])
            .current_dir(&profile_path)
            .output()
            .await
            .unwrap();

        let readme = String::from_utf8_lossy(&output.stdout);

        // Check frontmatter contains expected fields
        assert!(readme.contains("---"), "README should have frontmatter");
        assert!(readme.contains("name: bob"), "README should contain name");
        assert!(
            readme.contains("email: bob@example.com"),
            "README should contain email"
        );
        assert!(
            readme.contains("default_main_branch: main"),
            "README should contain default branch"
        );
        assert!(
            readme.contains("created_at:"),
            "README should contain created_at"
        );
    })
    .await;
}

/// Test that .profile repo contains authorized_keys file.
#[tokio::test(flavor = "current_thread")]
async fn test_profile_authorized_keys_created() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);
        let temp_dir = state.config.dir_root.as_path();

        let ssh_key = TestSshKey::generate_ed25519(temp_dir, "carol").unwrap();

        api.register("carol", "carol@test.com", "password123456789")
            .await
            .unwrap();

        // Add SSH key via settings
        api.update_settings(
            "carol",
            "carol@test.com",
            "",
            "",
            "main",
            &ssh_key.public_key_content,
        )
        .await
        .unwrap();

        // Use git to read the authorized_keys from the .profile repo
        let profile_path = state.config.git_root.join("carol").join(".profile");
        let output = tokio::process::Command::new("git")
            .args(["show", "main:authorized_keys"])
            .current_dir(&profile_path)
            .output()
            .await
            .unwrap();

        let keys = String::from_utf8_lossy(&output.stdout);
        assert!(
            keys.contains(&ssh_key.public_key_content),
            "authorized_keys should contain the SSH key"
        );
    })
    .await;
}

/// Test that .profile is hidden from user's project list.
#[tokio::test(flavor = "current_thread")]
async fn test_profile_hidden_from_projects() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);

        api.register("dave", "dave@test.com", "password123456789")
            .await
            .unwrap();

        // Create a regular project
        api.create_project("My Project", "A test project")
            .await
            .unwrap();

        // Check user's profile page - should show "My Project" but not ".profile"
        let profile_page = api.get_text("/~dave").await.unwrap();

        assert!(
            profile_page.contains("my-project") || profile_page.contains("My Project"),
            "User page should show regular projects"
        );
        assert!(
            !profile_page.contains(".profile"),
            "User page should NOT show .profile repo"
        );
    })
    .await;
}

/// Test that updating settings saves to .profile repo.
#[tokio::test(flavor = "current_thread")]
async fn test_settings_update_saves_to_profile() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);

        api.register("eve", "eve@test.com", "password123456789")
            .await
            .unwrap();

        // Update settings with new values
        api.update_settings(
            "eve",
            "eve@newmail.com",
            "https://eve.example.com",
            "Hello, I'm Eve!",
            "master",
            "",
        )
        .await
        .unwrap();

        // Read README.md from .profile repo
        let profile_path = state.config.git_root.join("eve").join(".profile");
        let output = tokio::process::Command::new("git")
            .args(["show", "main:README.md"])
            .current_dir(&profile_path)
            .output()
            .await
            .unwrap();

        let readme = String::from_utf8_lossy(&output.stdout);

        assert!(
            readme.contains("email: eve@newmail.com"),
            "README should have updated email"
        );
        assert!(
            readme.contains("website: https://eve.example.com"),
            "README should have website"
        );
        assert!(
            readme.contains("default_main_branch: master"),
            "README should have updated default branch"
        );
        assert!(
            readme.contains("Hello, I'm Eve!"),
            "README body should contain description"
        );
    })
    .await;
}

/// Test that user data loads correctly after profile is created.
#[tokio::test(flavor = "current_thread")]
async fn test_profile_data_loads_correctly() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);

        api.register("frank", "frank@test.com", "password123456789")
            .await
            .unwrap();

        // Update settings
        api.update_settings(
            "frank",
            "frank@test.com",
            "https://frank.dev",
            "This is my bio text",
            "main",
            "",
        )
        .await
        .unwrap();

        // Logout and login again to force reload from storage
        api.logout().await.unwrap();
        api.login("frank", "password123456789").await.unwrap();

        // Check settings page shows the saved values
        let settings_page = api.get_text("/settings").await.unwrap();

        assert!(
            settings_page.contains("https://frank.dev"),
            "Settings should show saved website"
        );
        assert!(
            settings_page.contains("This is my bio text"),
            "Settings should show saved description"
        );
    })
    .await;
}

/// Test graceful degradation when .profile repo is missing.
#[tokio::test(flavor = "current_thread")]
async fn test_graceful_degradation_missing_profile() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);

        api.register("grace", "grace@test.com", "password123456789")
            .await
            .unwrap();

        // Delete the .profile repo to simulate missing profile
        let profile_path = state.config.git_root.join("grace").join(".profile");
        tokio::fs::remove_dir_all(&profile_path).await.unwrap();

        // Logout and login - should still work with defaults
        api.logout().await.unwrap();
        api.login("grace", "password123456789").await.unwrap();

        // Settings page should still load
        let settings_page = api.get_text("/settings").await.unwrap();
        assert!(
            settings_page.contains("grace"),
            "Settings should load with default name"
        );

        // Saving settings should recreate .profile
        api.update_settings(
            "grace",
            "grace@test.com",
            "",
            "Recreated profile",
            "main",
            "",
        )
        .await
        .unwrap();

        assert!(
            profile_path.exists(),
            ".profile should be recreated after save"
        );
    })
    .await;
}

/// Test that JSON credentials file only contains sensitive data.
#[tokio::test(flavor = "current_thread")]
async fn test_json_contains_only_credentials() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);

        api.register("henry", "henry@test.com", "password123456789")
            .await
            .unwrap();

        // Update settings to add profile data
        api.update_settings(
            "henry",
            "henry@test.com",
            "https://henry.io",
            "Henry's description",
            "main",
            "",
        )
        .await
        .unwrap();

        // Read the JSON credentials file
        let json_path = state.config.git_root.join("!henry.json");
        let json_content = tokio::fs::read_to_string(&json_path).await.unwrap();

        // JSON should contain credentials
        assert!(
            json_content.contains("password_hash"),
            "JSON should have password_hash"
        );
        assert!(json_content.contains("\"id\""), "JSON should have id");
        assert!(json_content.contains("\"slug\""), "JSON should have slug");

        // JSON should NOT contain profile data (those go to .profile repo)
        assert!(
            !json_content.contains("henry.io"),
            "JSON should NOT contain website (goes to .profile)"
        );
        assert!(
            !json_content.contains("Henry's description"),
            "JSON should NOT contain description (goes to .profile)"
        );
    })
    .await;
}

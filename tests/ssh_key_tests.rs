//! Tests for SSH key management functionality.

mod common;

use common::{Api, TestSshKey, assertions::*, with_backend};

/// Test that SSH keys can be set via the settings page.
#[tokio::test(flavor = "current_thread")]
async fn test_ssh_key_management() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);
        let temp_dir = state.config.dir_root.as_path();

        let ssh_key = TestSshKey::generate_ed25519(temp_dir, "alice").unwrap();

        api.register("alice", "alice@test.com", "password123456789")
            .await
            .unwrap();

        api.update_settings(
            "alice",
            "alice@test.com",
            "",
            "",
            "main",
            &ssh_key.public_key_content,
        )
        .await
        .unwrap();

        api.assert_contains("/settings", &ssh_key.public_key_content)
            .await
            .unwrap();
    })
    .await;
}

/// Test that a user can have multiple SSH keys and both work.
#[tokio::test(flavor = "current_thread")]
async fn test_multiple_ssh_keys() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);
        let temp_dir = state.config.dir_root.as_path();

        // Generate two different SSH keys
        let key1 = TestSshKey::generate_ed25519(temp_dir, "alice_key1").unwrap();
        let key2 = TestSshKey::generate_rsa(temp_dir, "alice_key2").unwrap();

        // Register and add both keys (one per line)
        api.register("alice", "alice@test.com", "password123456789")
            .await
            .unwrap();

        let both_keys = format!("{}\n{}", key1.public_key_content, key2.public_key_content);
        api.update_settings("alice", "alice@test.com", "", "", "main", &both_keys)
            .await
            .unwrap();

        // Create a project
        api.create_project_with_access("Multi Key Test", "Testing multiple keys", "none")
            .await
            .unwrap();

        let ssh_url = format!(
            "ssh://alice@{}/~alice/multi-key-test",
            state.config.ssh_public_host
        );

        // Clone with first key
        let work1 = temp_dir.join("work1");
        std::fs::create_dir_all(&work1).unwrap();

        let git1 = common::GitHelper::new(
            work1.clone(),
            &key1.private_key_path,
            "Alice",
            "alice@test.com",
        );

        let result1 = git1.clone_ssh(&ssh_url, "repo").await.unwrap();
        assert_clone_success(&result1);

        // Clone with second key
        let work2 = temp_dir.join("work2");
        std::fs::create_dir_all(&work2).unwrap();

        let git2 = common::GitHelper::new(
            work2.clone(),
            &key2.private_key_path,
            "Alice",
            "alice@test.com",
        );

        let result2 = git2.clone_ssh(&ssh_url, "repo").await.unwrap();
        assert_clone_success(&result2);
    })
    .await;
}

/// Test that removing an SSH key revokes access for that key.
#[tokio::test(flavor = "current_thread")]
async fn test_ssh_key_removal() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);
        let temp_dir = state.config.dir_root.as_path();

        // Generate two SSH keys
        let key1 = TestSshKey::generate_ed25519(temp_dir, "alice_key1").unwrap();
        let key2 = TestSshKey::generate_ed25519(temp_dir, "alice_key2").unwrap();

        // Register with both keys
        api.register("alice", "alice@test.com", "password123456789")
            .await
            .unwrap();

        let both_keys = format!("{}\n{}", key1.public_key_content, key2.public_key_content);
        api.update_settings("alice", "alice@test.com", "", "", "main", &both_keys)
            .await
            .unwrap();

        // Create a project
        api.create_project_with_access("Key Removal Test", "Testing key removal", "none")
            .await
            .unwrap();

        let ssh_url = format!(
            "ssh://alice@{}/~alice/key-removal-test",
            state.config.ssh_public_host
        );

        // Verify key2 works initially
        let work1 = temp_dir.join("work1");
        std::fs::create_dir_all(&work1).unwrap();

        let git1 = common::GitHelper::new(
            work1.clone(),
            &key2.private_key_path,
            "Alice",
            "alice@test.com",
        );

        let result1 = git1.clone_ssh(&ssh_url, "repo").await.unwrap();
        assert_clone_success(&result1);

        // Remove key2, keep only key1
        api.update_settings(
            "alice",
            "alice@test.com",
            "",
            "",
            "main",
            &key1.public_key_content,
        )
        .await
        .unwrap();

        // Now key2 should NOT work
        let work2 = temp_dir.join("work2");
        std::fs::create_dir_all(&work2).unwrap();

        let git2 = common::GitHelper::new(
            work2.clone(),
            &key2.private_key_path,
            "Alice",
            "alice@test.com",
        );

        let result2 = git2.clone_ssh(&ssh_url, "repo2").await.unwrap();
        assert_clone_failure(&result2);

        // But key1 should still work
        let work3 = temp_dir.join("work3");
        std::fs::create_dir_all(&work3).unwrap();

        let git3 = common::GitHelper::new(
            work3.clone(),
            &key1.private_key_path,
            "Alice",
            "alice@test.com",
        );

        let result3 = git3.clone_ssh(&ssh_url, "repo3").await.unwrap();
        assert_clone_success(&result3);
    })
    .await;
}

/// Test that invalid SSH key formats are handled gracefully.
#[tokio::test(flavor = "current_thread")]
async fn test_invalid_ssh_key_format() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);
        let temp_dir = state.config.dir_root.as_path();

        api.register("alice", "alice@test.com", "password123456789")
            .await
            .unwrap();

        // Set an invalid SSH key (garbage data)
        api.update_settings(
            "alice",
            "alice@test.com",
            "",
            "",
            "main",
            "this-is-not-a-valid-ssh-key",
        )
        .await
        .unwrap();

        api.create_project_with_access("Invalid Key Test", "Testing invalid keys", "none")
            .await
            .unwrap();

        // Try to clone with a valid key that doesn't match the invalid stored key
        let real_key = TestSshKey::generate_ed25519(temp_dir, "real_key").unwrap();
        let work_dir = temp_dir.join("work");
        std::fs::create_dir_all(&work_dir).unwrap();

        let git = common::GitHelper::new(
            work_dir.clone(),
            &real_key.private_key_path,
            "Alice",
            "alice@test.com",
        );

        let ssh_url = format!(
            "ssh://alice@{}/~alice/invalid-key-test",
            state.config.ssh_public_host
        );

        // Should fail because the stored key is invalid/doesn't match
        let result = git.clone_ssh(&ssh_url, "repo").await.unwrap();
        assert_clone_failure(&result);

        // Now set a valid key and verify it works
        api.update_settings(
            "alice",
            "alice@test.com",
            "",
            "",
            "main",
            &real_key.public_key_content,
        )
        .await
        .unwrap();

        let work_dir2 = temp_dir.join("work2");
        std::fs::create_dir_all(&work_dir2).unwrap();

        let git2 = common::GitHelper::new(
            work_dir2.clone(),
            &real_key.private_key_path,
            "Alice",
            "alice@test.com",
        );

        let result2 = git2.clone_ssh(&ssh_url, "repo").await.unwrap();
        assert_clone_success(&result2);
    })
    .await;
}

//! Tests for repository access control.

mod common;

use common::{Api, TestSshKey, TestUser, assertions::*, with_backend};

/// Test that other users and anonymous users cannot clone a private repo.
#[tokio::test(flavor = "current_thread")]
async fn test_private_repo_access_denied() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);
        let temp_dir = state.config.dir_root.as_path();

        // Alice creates a private project
        let alice = TestUser::create(&api, temp_dir, "alice", &state.config.ssh_public_host)
            .await
            .unwrap();

        api.create_project_with_access("Private Repo", "Secret stuff", "none")
            .await
            .unwrap();
        api.logout().await.unwrap();

        // Bob registers and tries to clone Alice's private repo
        let bob = TestUser::create(&api, temp_dir, "bob", &state.config.ssh_public_host)
            .await
            .unwrap();

        let (work_dir, bob_git) = bob.setup_workspace(temp_dir, "bob_workspace").unwrap();

        // Clone should FAIL (Bob doesn't have access)
        let output = bob_git
            .clone_ssh(&bob.ssh_url_for("alice", "private-repo"), "stolen-repo")
            .await
            .unwrap();
        assert_clone_failure(&output);

        // Anonymous access should also fail
        let anon_key = TestSshKey::generate_ed25519(temp_dir, "anon_attempt").unwrap();
        let anon_git = common::GitHelper::new(
            work_dir.clone(),
            &anon_key.private_key_path,
            "Anonymous",
            "anon@test.com",
        );

        let anon_output = anon_git
            .clone_ssh(&alice.anon_ssh_url("alice", "private-repo"), "anon-stolen")
            .await
            .unwrap();
        assert_clone_failure(&anon_output);

        // Web access should also return not found (private repos are hidden)
        let response = api.get_raw("/~alice/private-repo").await.unwrap();
        assert_eq!(
            response.status().as_u16(),
            404,
            "Private repo should return 404 for unauthorized web access"
        );
    })
    .await;
}

/// Test that any authenticated user can push to a public-write repo.
#[tokio::test(flavor = "current_thread")]
async fn test_public_write_access() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);
        let temp_dir = state.config.dir_root.as_path();

        // Alice creates a public-write project
        let alice = TestUser::create(&api, temp_dir, "alice", &state.config.ssh_public_host)
            .await
            .unwrap();

        api.create_project_with_access("Open Source", "Everyone can contribute", "write")
            .await
            .unwrap();

        // Alice pushes an initial commit
        let (alice_work, alice_git) = alice.setup_workspace(temp_dir, "alice_work").unwrap();

        alice_git
            .clone_ssh(&alice.ssh_url("open-source"), "open-source")
            .await
            .unwrap();
        let alice_repo = alice_work.join("open-source");
        alice_git.configure_identity(&alice_repo).await.unwrap();
        alice_git
            .create_commit(&alice_repo, "README.md", "# Open Source Project\n", "Init")
            .await
            .unwrap();
        let push_result = alice_git
            .push_ssh(&alice_repo, "origin", "main")
            .await
            .unwrap();
        assert_push_success(&push_result);

        api.logout().await.unwrap();

        // Bob registers and can clone and push to the public-write repo
        let bob = TestUser::create(&api, temp_dir, "bob", &state.config.ssh_public_host)
            .await
            .unwrap();

        let (bob_work, bob_git) = bob.setup_workspace(temp_dir, "bob_work").unwrap();

        // Clone should work
        let clone_result = bob_git
            .clone_ssh(&bob.ssh_url_for("alice", "open-source"), "open-source")
            .await
            .unwrap();
        assert_clone_success(&clone_result);

        let bob_repo = bob_work.join("open-source");
        bob_git.configure_identity(&bob_repo).await.unwrap();
        bob_git
            .create_commit(
                &bob_repo,
                "CONTRIBUTING.md",
                "# Contributing\n\nPRs welcome!\n",
                "Add contributing guide",
            )
            .await
            .unwrap();

        // Push should succeed (public write access)
        let push_result = bob_git.push_ssh(&bob_repo, "origin", "main").await.unwrap();
        assert_push_success(&push_result);

        // Verify commit is visible on web
        api.assert_contains("/~alice/open-source", "Add contributing guide")
            .await
            .unwrap();
    })
    .await;
}

/// Test that public read repos allow clone but not push for non-owners.
#[tokio::test(flavor = "current_thread")]
async fn test_public_read_no_push() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);
        let temp_dir = state.config.dir_root.as_path();

        // Alice creates a public-read project
        let alice = TestUser::create(&api, temp_dir, "alice", &state.config.ssh_public_host)
            .await
            .unwrap();

        api.create_project_with_access("Read Only", "Public read, no write", "read")
            .await
            .unwrap();

        // Alice pushes initial content
        let (alice_work, alice_git) = alice.setup_workspace(temp_dir, "alice_work").unwrap();

        alice_git
            .clone_ssh(&alice.ssh_url("read-only"), "read-only")
            .await
            .unwrap();
        let alice_repo = alice_work.join("read-only");
        alice_git.configure_identity(&alice_repo).await.unwrap();
        alice_git
            .create_commit(&alice_repo, "README.md", "# Read Only\n", "Init")
            .await
            .unwrap();
        alice_git
            .push_ssh(&alice_repo, "origin", "main")
            .await
            .unwrap();

        api.logout().await.unwrap();

        // Bob registers and can clone
        let bob = TestUser::create(&api, temp_dir, "bob", &state.config.ssh_public_host)
            .await
            .unwrap();

        let (bob_work, bob_git) = bob.setup_workspace(temp_dir, "bob_work").unwrap();

        // Clone should work
        let clone_result = bob_git
            .clone_ssh(&bob.ssh_url_for("alice", "read-only"), "read-only")
            .await
            .unwrap();
        assert_clone_success(&clone_result);

        // But push should fail
        let bob_repo = bob_work.join("read-only");
        bob_git.configure_identity(&bob_repo).await.unwrap();
        bob_git
            .create_commit(&bob_repo, "HACK.md", "# Hacked!\n", "Unauthorized commit")
            .await
            .unwrap();

        let push_result = bob_git.push_ssh(&bob_repo, "origin", "main").await.unwrap();
        assert_push_failure(&push_result);
    })
    .await;
}

/// Test that changing project visibility revokes access appropriately.
#[tokio::test(flavor = "current_thread")]
async fn test_change_project_visibility() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);
        let temp_dir = state.config.dir_root.as_path();

        // Alice creates a public project
        let alice = TestUser::create(&api, temp_dir, "alice", &state.config.ssh_public_host)
            .await
            .unwrap();

        api.create_project_with_access("Visibility Test", "Testing visibility change", "read")
            .await
            .unwrap();

        // Alice pushes content
        let (alice_work, alice_git) = alice.setup_workspace(temp_dir, "alice_work").unwrap();

        alice_git
            .clone_ssh(&alice.ssh_url("visibility-test"), "visibility-test")
            .await
            .unwrap();
        let alice_repo = alice_work.join("visibility-test");
        alice_git.configure_identity(&alice_repo).await.unwrap();
        alice_git
            .create_commit(&alice_repo, "README.md", "# Test\n", "Init")
            .await
            .unwrap();
        alice_git
            .push_ssh(&alice_repo, "origin", "main")
            .await
            .unwrap();

        // Bob can clone while public (using anon key since he's not registered)
        let bob_key = TestSshKey::generate_ed25519(temp_dir, "bob").unwrap();
        let bob_work = temp_dir.join("bob_work");
        std::fs::create_dir_all(&bob_work).unwrap();

        let bob_git = common::GitHelper::new(
            bob_work.clone(),
            &bob_key.private_key_path,
            "Bob",
            "bob@test.com",
        );

        let bob_ssh_url = alice.anon_ssh_url("alice", "visibility-test");

        let clone_result = bob_git
            .clone_ssh(&bob_ssh_url, "visibility-test")
            .await
            .unwrap();
        assert_clone_success(&clone_result);

        // Alice changes project to private via settings
        api.update_project_settings(
            "alice",
            "visibility-test",
            "Visibility Test",
            "Now private",
            "none",
            "main",
            "",
        )
        .await
        .unwrap();

        // Now Bob cannot clone (try fresh clone to different dir)
        let bob_work2 = temp_dir.join("bob_work2");
        std::fs::create_dir_all(&bob_work2).unwrap();

        let bob_git2 = common::GitHelper::new(
            bob_work2.clone(),
            &bob_key.private_key_path,
            "Bob",
            "bob@test.com",
        );

        let clone_result2 = bob_git2
            .clone_ssh(&bob_ssh_url, "visibility-test")
            .await
            .unwrap();
        assert_clone_failure(&clone_result2);
    })
    .await;
}

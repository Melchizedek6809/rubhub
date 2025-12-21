//! Tests for SSH clone and push operations.

mod common;

use common::{assertions::*, with_backend, Api, KeyType, TestUser};
use tokio::process::Command;

/// Test that an owner can clone their private repo via SSH with an Ed25519 key.
#[tokio::test(flavor = "current_thread")]
async fn test_ssh_clone_private_repo_owner_ed25519() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);
        let temp_dir = state.config.dir_root.as_path();

        let user =
            TestUser::create(&api, temp_dir, "alice", &state.config.ssh_public_host).await.unwrap();

        api.create_project_with_access("My Private Project", "A private project", "none")
            .await
            .unwrap();

        let (work_dir, git) = user.setup_workspace(temp_dir, "workspaces").unwrap();

        let output = git
            .clone_ssh(&user.ssh_url("my-private-project"), "cloned-repo")
            .await
            .unwrap();
        assert_clone_success(&output);
        assert!(work_dir.join("cloned-repo").exists());
    })
    .await;
}

/// Test that an owner can clone their private repo via SSH with an RSA key.
#[tokio::test(flavor = "current_thread")]
async fn test_ssh_clone_private_repo_owner_rsa() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);
        let temp_dir = state.config.dir_root.as_path();

        let user = TestUser::create_with_key_type(
            &api,
            temp_dir,
            "alice",
            &state.config.ssh_public_host,
            KeyType::Rsa,
        )
        .await
        .unwrap();

        api.create_project_with_access("My Private Project", "A private project", "none")
            .await
            .unwrap();

        let (work_dir, git) = user.setup_workspace(temp_dir, "workspaces").unwrap();

        let output = git
            .clone_ssh(&user.ssh_url("my-private-project"), "cloned-repo")
            .await
            .unwrap();
        assert_clone_success(&output);
        assert!(work_dir.join("cloned-repo").exists());
    })
    .await;
}

/// Test that an owner can commit and push to their repo.
#[tokio::test(flavor = "current_thread")]
async fn test_ssh_commit_and_push() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);
        let temp_dir = state.config.dir_root.as_path();

        let user =
            TestUser::create(&api, temp_dir, "alice", &state.config.ssh_public_host).await.unwrap();

        // Create project (public read so we can verify on web)
        api.create_project_with_access("Push Test", "Testing push", "read")
            .await
            .unwrap();

        let (work_dir, git) = user.setup_workspace(temp_dir, "workspaces").unwrap();

        let clone_output = git
            .clone_ssh(&user.ssh_url("push-test"), "push-test")
            .await
            .unwrap();
        assert_clone_success(&clone_output);

        let repo_dir = work_dir.join("push-test");
        git.configure_identity(&repo_dir).await.unwrap();

        let commit_output = git
            .create_commit(&repo_dir, "README.md", "# Hello World\n", "Initial commit")
            .await
            .unwrap();
        assert_success(&commit_output, "Commit");

        let push_output = git.push_ssh(&repo_dir, "origin", "main").await.unwrap();
        assert_push_success(&push_output);

        api.assert_contains("/~alice/push-test", "Initial commit")
            .await
            .unwrap();
    })
    .await;
}

/// Test that anonymous users can clone public repos via SSH.
#[tokio::test(flavor = "current_thread")]
async fn test_anonymous_ssh_clone_public_repo() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);
        let temp_dir = state.config.dir_root.as_path();

        // Alice creates a public project and pushes content
        let alice =
            TestUser::create(&api, temp_dir, "alice", &state.config.ssh_public_host).await.unwrap();

        api.create_project_with_access("Public Project", "A public project", "read")
            .await
            .unwrap();

        // Alice pushes initial content
        let (alice_work, alice_git) = alice.setup_workspace(temp_dir, "alice_work").unwrap();

        alice_git
            .clone_ssh(&alice.ssh_url("public-project"), "public-project")
            .await
            .unwrap();
        let alice_repo = alice_work.join("public-project");
        alice_git.configure_identity(&alice_repo).await.unwrap();
        alice_git
            .create_commit(&alice_repo, "README.md", "# Public\n", "Init")
            .await
            .unwrap();
        alice_git
            .push_ssh(&alice_repo, "origin", "main")
            .await
            .unwrap();

        // Anonymous user (with any SSH key) can clone via "anon" username
        let anon_key = common::TestSshKey::generate_ed25519(temp_dir, "anon_user").unwrap();
        let anon_work = temp_dir.join("anon_work");
        std::fs::create_dir_all(&anon_work).unwrap();

        let anon_git = common::GitHelper::new(
            anon_work.clone(),
            &anon_key.private_key_path,
            "Anonymous",
            "anon@test.com",
        );

        let anon_ssh_url = alice.anon_ssh_url("alice", "public-project");

        let clone_result = anon_git
            .clone_ssh(&anon_ssh_url, "public-project")
            .await
            .unwrap();
        assert_clone_success(&clone_result);
        assert!(anon_work.join("public-project/README.md").exists());
    })
    .await;
}

/// Test that HTTP clone works for public repos.
#[tokio::test(flavor = "current_thread")]
async fn test_http_clone_public_repo() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);
        let temp_dir = state.config.dir_root.as_path();

        // Create user and public project with content
        let alice =
            TestUser::create(&api, temp_dir, "alice", &state.config.ssh_public_host).await.unwrap();

        api.create_project_with_access("Public Repo", "A public project", "read")
            .await
            .unwrap();

        // Push initial content via SSH (so there's something to clone)
        let (alice_work, git) = alice.setup_workspace(temp_dir, "alice_work").unwrap();

        git.clone_ssh(&alice.ssh_url("public-repo"), "public-repo")
            .await
            .unwrap();
        let repo_dir = alice_work.join("public-repo");
        git.configure_identity(&repo_dir).await.unwrap();
        git.create_commit(&repo_dir, "README.md", "# Public Project\n", "Initial")
            .await
            .unwrap();
        let push_result = git.push_ssh(&repo_dir, "origin", "main").await.unwrap();
        assert_push_success(&push_result);

        // HTTP clone (no authentication needed)
        let http_work = temp_dir.join("http_clone");
        std::fs::create_dir_all(&http_work).unwrap();

        let http_url = format!("{}/~alice/public-repo", state.config.base_url);

        let clone_output = Command::new("git")
            .current_dir(&http_work)
            .arg("clone")
            .arg(&http_url)
            .arg("cloned")
            .output()
            .await
            .unwrap();

        assert_clone_success(&clone_output);
        assert!(http_work.join("cloned/README.md").exists());
    })
    .await;
}

/// Test that HTTP clone fails for private repos.
#[tokio::test(flavor = "current_thread")]
async fn test_http_clone_private_repo_blocked() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);
        let temp_dir = state.config.dir_root.as_path();

        // Create private project
        TestUser::create(&api, temp_dir, "alice", &state.config.ssh_public_host)
            .await
            .unwrap();

        api.create_project_with_access("Private", "Secret project", "none")
            .await
            .unwrap();

        api.logout().await.unwrap();

        // HTTP clone should fail for private repo
        let http_work = temp_dir.join("http_clone_private");
        std::fs::create_dir_all(&http_work).unwrap();

        let http_url = format!("{}/~alice/private", state.config.base_url);

        let clone_output = Command::new("git")
            .current_dir(&http_work)
            .arg("clone")
            .arg(&http_url)
            .arg("stolen")
            .output()
            .await
            .unwrap();

        assert_clone_failure(&clone_output);
    })
    .await;
}

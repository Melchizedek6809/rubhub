//! Tests for project metadata and fork behavior.

mod common;

use std::process::Command;

use common::{Api, TestUser, assertions::*, with_backend};

#[tokio::test(flavor = "current_thread")]
async fn project_metadata_is_written_to_meta_info() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);

        api.register("alice", "alice@example.com", "alicepassword123")
            .await
            .unwrap();
        api.create_project_with_access("Meta Project", "", "read")
            .await
            .unwrap();
        api.update_project_settings(
            "alice",
            "meta-project",
            "Meta Project",
            "Stored in Git",
            "read",
            "main",
            "",
        )
        .await
        .unwrap();

        let output = Command::new("git")
            .arg("-C")
            .arg(state.config.git_root.join("alice/meta-project"))
            .args(["show", "meta/info:README.md"])
            .output()
            .unwrap();

        assert!(output.status.success());
        let info = String::from_utf8_lossy(&output.stdout);
        assert!(info.contains("name: Meta Project"));
        assert!(info.contains("description: Stored in Git"));
        assert!(info.contains("default_branch: main"));
        assert!(info.contains("canonical:"));

        let body = api.get_text("/~alice/meta-project").await.unwrap();
        assert!(body.contains("Stored in Git"));
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn fork_copies_code_tags_and_info_but_not_talk() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);
        let temp_dir = state.config.dir_root.as_path();

        let alice = TestUser::create(&api, temp_dir, "alice", &state.config.ssh_public_host)
            .await
            .unwrap();
        api.create_project_with_access("Source Project", "", "read")
            .await
            .unwrap();

        let (alice_work, alice_git) = alice.setup_workspace(temp_dir, "alice_work").unwrap();
        alice_git
            .clone_ssh(&alice.ssh_url("source-project"), "source-project")
            .await
            .unwrap();
        let alice_repo = alice_work.join("source-project");
        alice_git.configure_identity(&alice_repo).await.unwrap();
        alice_git
            .create_commit(&alice_repo, "README.md", "# Source\n", "Init")
            .await
            .unwrap();
        let push = alice_git
            .push_ssh(&alice_repo, "origin", "main")
            .await
            .unwrap();
        assert_push_success(&push);

        Command::new("git")
            .current_dir(&alice_repo)
            .args(["tag", "v1"])
            .status()
            .unwrap();
        let tag_push = alice_git
            .push_ssh(&alice_repo, "origin", "v1")
            .await
            .unwrap();
        assert_push_success(&tag_push);

        let commit = Command::new("git")
            .arg("-C")
            .arg(state.config.git_root.join("alice/source-project"))
            .args(["rev-parse", "refs/heads/main"])
            .output()
            .unwrap();
        assert!(commit.status.success());
        let commit = String::from_utf8_lossy(&commit.stdout).trim().to_owned();
        for ref_name in ["refs/notes/review", "refs/heads/meta/custom"] {
            let status = Command::new("git")
                .arg("-C")
                .arg(state.config.git_root.join("alice/source-project"))
                .args(["update-ref", ref_name, &commit])
                .status()
                .unwrap();
            assert!(status.success());
        }

        api.create_issue(
            "alice",
            "source-project",
            "Do not copy",
            "Source-only issue",
        )
        .await
        .unwrap();
        api.logout().await.unwrap();

        TestUser::create(&api, temp_dir, "bob", &state.config.ssh_public_host)
            .await
            .unwrap();
        api.fork_project("alice", "source-project", "Bob Fork", "read")
            .await
            .unwrap();

        assert!(state.config.git_root.join("bob/bob-fork").exists());

        let refs = Command::new("git")
            .arg("-C")
            .arg(state.config.git_root.join("bob/bob-fork"))
            .args(["for-each-ref", "--format=%(refname)"])
            .output()
            .unwrap();
        assert!(refs.status.success());
        let refs = String::from_utf8_lossy(&refs.stdout);
        assert!(refs.contains("refs/heads/main"));
        assert!(refs.contains("refs/tags/v1"));
        assert!(refs.contains("refs/heads/meta/info"));
        assert!(!refs.contains("refs/heads/meta/talk"));
        assert!(!refs.contains("refs/heads/meta/custom"));
        assert!(!refs.contains("refs/notes/review"));

        let info = Command::new("git")
            .arg("-C")
            .arg(state.config.git_root.join("bob/bob-fork"))
            .args(["show", "meta/info:README.md"])
            .output()
            .unwrap();
        assert!(info.status.success());
        let info = String::from_utf8_lossy(&info.stdout);
        assert!(info.contains("name: Bob Fork"));
        assert!(info.contains("canonical:"));
        assert!(info.contains("upstream:"));
        assert!(info.contains("/~alice/source-project"));

        let body = api.get_text("/~bob/bob-fork").await.unwrap();
        assert!(body.contains("Forked from"));
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn private_projects_cannot_be_forked() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);
        let temp_dir = state.config.dir_root.as_path();

        TestUser::create(&api, temp_dir, "alice", &state.config.ssh_public_host)
            .await
            .unwrap();
        api.create_project_with_access("Private Source", "", "none")
            .await
            .unwrap();

        let body = api.get_text("/~alice/private-source").await.unwrap();
        assert!(!body.contains("Fork"));

        let response = api
            .fork_project_raw("alice", "private-source", "Private Fork", "none")
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 404);
        assert!(!state.config.git_root.join("alice/private-fork").exists());
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn meta_branches_cannot_be_default_branches() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);

        api.register("alice", "alice@example.com", "alicepassword123")
            .await
            .unwrap();
        api.create_project_with_access("Branch Project", "", "read")
            .await
            .unwrap();

        let response = api
            .update_project_settings(
                "alice",
                "branch-project",
                "Branch Project",
                "",
                "read",
                "meta/info",
                "",
            )
            .await
            .unwrap();
        let body = response.text().await.unwrap();
        assert!(body.contains("Branch names starting with meta/ are reserved."));
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn meta_branches_are_hidden_from_code_navigation() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);

        api.register("alice", "alice@example.com", "alicepassword123")
            .await
            .unwrap();
        api.create_project_with_access("Empty Project", "", "read")
            .await
            .unwrap();

        let info = Command::new("git")
            .arg("-C")
            .arg(state.config.git_root.join("alice/empty-project"))
            .args(["show-ref", "--verify", "refs/heads/meta/info"])
            .output()
            .unwrap();
        assert!(info.status.success());

        let body = api.get_text("/~alice/empty-project").await.unwrap();
        assert!(body.contains("Empty Repo"));
        assert!(!body.contains("meta/info"));
    })
    .await;
}

mod common;

use common::with_backend;
use rubhub::{AccessType, RepoEvent, RepoEventInfo};
use time::OffsetDateTime;

#[tokio::test(flavor = "current_thread")]
async fn emitted_branch_event_reaches_subscribers() {
    with_backend(|state| async move {
        let mut events = state.event_tx.subscribe();

        state.emit_event(RepoEvent::BranchUpdated {
            info: RepoEventInfo {
                owner: "testuser".to_string(),
                project: "test-project".to_string(),
                commit_hash: "abc123".to_string(),
                timestamp: OffsetDateTime::now_utc(),
            },
            branch: "main".to_string(),
        });

        let received = events.recv().await.unwrap();
        match received {
            RepoEvent::BranchUpdated { info, branch } => {
                assert_eq!(info.owner, "testuser");
                assert_eq!(info.project, "test-project");
                assert_eq!(info.commit_hash, "abc123");
                assert_eq!(branch, "main");
            }
            other => panic!("expected branch event, got {other:?}"),
        }
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn emitted_create_event_reaches_subscribers() {
    with_backend(|state| async move {
        let mut events = state.event_tx.subscribe();

        state.emit_event(RepoEvent::RepositoryCreated {
            owner: "testuser".to_string(),
            project: "test-project".to_string(),
            public_access: AccessType::Read,
            timestamp: OffsetDateTime::now_utc(),
        });

        let received = events.recv().await.unwrap();
        match received {
            RepoEvent::RepositoryCreated {
                owner,
                project,
                public_access,
                ..
            } => {
                assert_eq!(owner, "testuser");
                assert_eq!(project, "test-project");
                assert_eq!(public_access, AccessType::Read);
            }
            other => panic!("expected create event, got {other:?}"),
        }
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn emitted_delete_event_reaches_subscribers() {
    with_backend(|state| async move {
        let mut events = state.event_tx.subscribe();

        state.emit_event(RepoEvent::RepositoryDeleted {
            owner: "testuser".to_string(),
            project: "test-project".to_string(),
            public_access: AccessType::Read,
            timestamp: OffsetDateTime::now_utc(),
        });

        let received = events.recv().await.unwrap();
        match received {
            RepoEvent::RepositoryDeleted {
                owner,
                project,
                public_access,
                ..
            } => {
                assert_eq!(owner, "testuser");
                assert_eq!(project, "test-project");
                assert_eq!(public_access, AccessType::Read);
            }
            other => panic!("expected delete event, got {other:?}"),
        }
    })
    .await;
}

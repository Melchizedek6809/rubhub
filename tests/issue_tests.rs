mod common;

use common::{Api, with_backend};

#[tokio::test(flavor = "current_thread")]
async fn test_issue_workflow() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);

        // Register and create a project
        api.register("alice", "alice@example.com", "password123456789")
            .await
            .unwrap();

        api.create_project("Test Project", "A project for testing issues")
            .await
            .unwrap();

        // Verify issues list is empty initially
        api.assert_contains("/~alice/test-project/issues", "No issues yet")
            .await
            .unwrap();

        // Create a new issue
        api.create_issue(
            "alice",
            "test-project",
            "First Issue",
            "This is the description of my first issue.",
        )
        .await
        .unwrap();

        // Verify issue appears in the list
        let issues_page = api.get_text("/~alice/test-project/issues").await.unwrap();
        assert!(
            issues_page.contains("First Issue"),
            "Issue title should appear in list"
        );
        assert!(
            issues_page.contains("opened by alice"),
            "Author should appear in list"
        );
        assert!(
            issues_page.contains("status-open"),
            "Issue should have open status"
        );

        // Extract issue directory name from the list page link
        let issue_dir =
            extract_issue_dir(&issues_page, "First Issue").expect("Should find issue link in list");

        // View the single issue
        let issue_path = format!("/~alice/test-project/issues/{}", issue_dir);
        let issue_page = api.get_text(&issue_path).await.unwrap();
        assert!(
            issue_page.contains("First Issue"),
            "Issue title should appear on view page"
        );
        assert!(
            issue_page.contains("This is the description of my first issue"),
            "Issue description should appear"
        );
        assert!(
            issue_page.contains("status-open"),
            "Issue should show open status"
        );

        // Add a comment to the issue
        api.add_issue_comment(
            "alice",
            "test-project",
            &issue_dir,
            "This is a follow-up comment.",
            None,
        )
        .await
        .unwrap();

        // Verify comment appears on the issue view
        let issue_page = api.get_text(&issue_path).await.unwrap();
        assert!(
            issue_page.contains("This is a follow-up comment"),
            "Comment should appear on issue page"
        );

        // Close the issue (mark as completed)
        api.add_issue_comment(
            "alice",
            "test-project",
            &issue_dir,
            "Closing this issue as completed.",
            Some("completed"),
        )
        .await
        .unwrap();

        // Verify status changed on single issue view
        let issue_page = api.get_text(&issue_path).await.unwrap();
        assert!(
            issue_page.contains("status-completed"),
            "Issue should show completed status"
        );
        assert!(
            issue_page.contains("changed status to completed"),
            "Status change should be noted in comment"
        );

        // By default, completed issues should NOT appear in the list
        let issues_page = api.get_text("/~alice/test-project/issues").await.unwrap();
        assert!(
            !issues_page.contains("First Issue"),
            "Completed issue should not appear in default list view"
        );
        assert!(
            issues_page.contains("Show all"),
            "Should show 'Show all' button when filtering"
        );

        // With showCompleted=true, the issue should appear
        let issues_page = api
            .get_text("/~alice/test-project/issues?showCompleted=true&showCancelled=true")
            .await
            .unwrap();
        assert!(
            issues_page.contains("First Issue"),
            "Completed issue should appear when showCompleted=true"
        );
        assert!(
            issues_page.contains("status-completed"),
            "List should show completed status"
        );
        assert!(
            issues_page.contains("Show open only"),
            "Should show 'Show open only' button when showing all"
        );
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_issue_reopen() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);

        api.register("bob", "bob@example.com", "password123456789")
            .await
            .unwrap();

        api.create_project("Reopen Test", "Testing issue reopening")
            .await
            .unwrap();

        api.create_issue("bob", "reopen-test", "Bug Report", "Found a bug.")
            .await
            .unwrap();

        let issues_page = api.get_text("/~bob/reopen-test/issues").await.unwrap();
        let issue_dir =
            extract_issue_dir(&issues_page, "Bug Report").expect("Should find issue link");

        // Close the issue
        api.add_issue_comment(
            "bob",
            "reopen-test",
            &issue_dir,
            "Fixed!",
            Some("completed"),
        )
        .await
        .unwrap();

        let issue_path = format!("/~bob/reopen-test/issues/{}", issue_dir);
        let issue_page = api.get_text(&issue_path).await.unwrap();
        assert!(
            issue_page.contains("status-completed"),
            "Issue should be completed"
        );

        // Reopen the issue
        api.add_issue_comment(
            "bob",
            "reopen-test",
            &issue_dir,
            "Actually, the bug is still there.",
            Some("open"),
        )
        .await
        .unwrap();

        let issue_page = api.get_text(&issue_path).await.unwrap();
        assert!(
            issue_page.contains("status-open"),
            "Issue should be open again"
        );
        assert!(
            issue_page.contains("changed status to open"),
            "Reopen should be noted"
        );
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_issue_cancelled() {
    with_backend(|state| async move {
        let api = Api::new(&state.config.base_url);

        api.register("carol", "carol@example.com", "password123456789")
            .await
            .unwrap();

        api.create_project("Cancel Test", "Testing issue cancellation")
            .await
            .unwrap();

        api.create_issue(
            "carol",
            "cancel-test",
            "Wont Fix",
            "This will be cancelled.",
        )
        .await
        .unwrap();

        let issues_page = api.get_text("/~carol/cancel-test/issues").await.unwrap();
        let issue_dir =
            extract_issue_dir(&issues_page, "Wont Fix").expect("Should find issue link");

        // Cancel/close the issue
        api.add_issue_comment(
            "carol",
            "cancel-test",
            &issue_dir,
            "Not going to fix this.",
            Some("cancelled"),
        )
        .await
        .unwrap();

        let issue_path = format!("/~carol/cancel-test/issues/{}", issue_dir);
        let issue_page = api.get_text(&issue_path).await.unwrap();
        assert!(
            issue_page.contains("status-cancelled"),
            "Issue should be cancelled"
        );

        // By default, cancelled issues should NOT appear in the list
        let issues_page = api.get_text("/~carol/cancel-test/issues").await.unwrap();
        assert!(
            !issues_page.contains("Wont Fix"),
            "Cancelled issue should not appear in default list view"
        );

        // With showCancelled=true, the issue should appear
        let issues_page = api
            .get_text("/~carol/cancel-test/issues?showCancelled=true")
            .await
            .unwrap();
        assert!(
            issues_page.contains("Wont Fix"),
            "Cancelled issue should appear when showCancelled=true"
        );
        assert!(
            issues_page.contains("status-cancelled"),
            "List should show cancelled status"
        );
    })
    .await;
}

/// Extract the issue directory name from the issues list HTML.
/// Looks for links like: href="/~owner/project/issues/DIRNAME"
fn extract_issue_dir(html: &str, issue_title: &str) -> Option<String> {
    // Find the issue title in the HTML
    let title_pos = html.find(issue_title)?;

    // Search backwards for the href containing the issue path
    let before_title = &html[..title_pos];
    let href_needle = "/issues/";
    let href_start = before_title.rfind(href_needle)? + href_needle.len();

    // Extract up to the closing quote
    let rest = &html[href_start..];
    let end = rest.find('"')?;

    Some(rest[..end].to_string())
}

use anyhow::{Result, anyhow};
use time::OffsetDateTime;

use crate::{
    GlobalState, Project, User,
    models::{CommentFrontmatter, Issue, IssueComment, IssueStatus, IssueSummary},
    services::{
        repository::{
            CommitParams, add_file_to_branch, branch_exists, create_orphan_branch, get_git_file,
            get_git_tree,
        },
        validation::slugify,
    },
};

const ISSUES_BRANCH: &str = "meta/talk";
const ISSUES_DIR: &str = "issues";

/// Parse YAML frontmatter from markdown content
fn parse_frontmatter(content: &str) -> Result<(CommentFrontmatter, String)> {
    if !content.starts_with("---\n") {
        return Err(anyhow!("Missing frontmatter"));
    }

    let rest = &content[4..];
    let end = rest
        .find("\n---\n")
        .or_else(|| rest.find("\n---"))
        .ok_or_else(|| anyhow!("Invalid frontmatter"))?;
    let yaml = &rest[..end];

    // Find where body starts (after the closing ---)
    let body_start = if rest[end..].starts_with("\n---\n") {
        end + 5
    } else {
        end + 4
    };
    let body = if body_start < rest.len() {
        rest[body_start..].trim()
    } else {
        ""
    };

    let frontmatter: CommentFrontmatter = serde_yaml::from_str(yaml)?;
    Ok((frontmatter, body.to_string()))
}

/// Generate timestamp prefix for filenames: YYYY-MM-DD-HH-MM-SS-mmm
fn timestamp_prefix(dt: OffsetDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}-{:02}-{:02}-{:02}-{:03}",
        dt.year(),
        dt.month() as u8,
        dt.day(),
        dt.hour(),
        dt.minute(),
        dt.second(),
        dt.millisecond()
    )
}

/// Generate unique issue directory name
fn generate_issue_dir_name(title: &str, existing: &[String]) -> String {
    let slug = slugify(title);
    let now = OffsetDateTime::now_utc();
    let prefix = timestamp_prefix(now);
    let base_name = format!("{}-{}", prefix, slug);

    if !existing.contains(&base_name) {
        return base_name;
    }

    // Add counter suffix if exists
    for i in 1..1000 {
        let name = format!("{}-{}", base_name, i);
        if !existing.contains(&name) {
            return name;
        }
    }
    // Fallback with random suffix
    format!("{}-{}", base_name, rand_suffix())
}

/// Generate unique comment filename
fn generate_comment_filename(author: &str, existing: &[String]) -> String {
    let now = OffsetDateTime::now_utc();
    let prefix = timestamp_prefix(now);
    let base_name = format!("{}-{}.md", prefix, author);

    if !existing.contains(&base_name) {
        return base_name;
    }

    for i in 1..1000 {
        let name = format!("{}-{}-{}.md", prefix, author, i);
        if !existing.contains(&name) {
            return name;
        }
    }
    format!("{}-{}-{}.md", prefix, author, rand_suffix())
}

fn rand_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("{:08x}", nanos)
}

/// Ensure the issues branch exists, create if not
pub async fn ensure_issues_branch(
    state: &GlobalState,
    user: &User,
    project: &Project,
) -> Result<()> {
    if branch_exists(state, &user.slug, &project.slug, ISSUES_BRANCH).await {
        return Ok(());
    }

    let readme_content = format!(
        "# RubHub Issues\n\nThis branch contains issues for the {} project.\n\n\
        To create a new issue you must create a new directory in `issues/` containing a single markdown file with the initial comment (must also contain a title in the frontmatter).\n",
        project.name
    );

    create_orphan_branch(CommitParams {
        state,
        user_name: &user.slug,
        project_slug: &project.slug,
        branch_name: ISSUES_BRANCH,
        file_path: "README.md",
        file_content: &readme_content,
        commit_message: "Initialize issues branch",
        author_name: "RubHub",
        author_email: "noreply@rubhub.net",
    })
    .await?;

    Ok(())
}

/// List all issues for a project
pub async fn list_issues(
    state: &GlobalState,
    user_slug: &str,
    project_slug: &str,
) -> Result<Vec<IssueSummary>> {
    // Check if branch exists
    if !branch_exists(state, user_slug, project_slug, ISSUES_BRANCH).await {
        return Ok(vec![]);
    }

    // Get tree of issues/ directory
    let tree = match get_git_tree(state, user_slug, project_slug, ISSUES_BRANCH, ISSUES_DIR).await {
        Ok(t) => t,
        Err(_) => return Ok(vec![]), // issues/ directory doesn't exist yet
    };

    let mut summaries = vec![];

    for entry in tree {
        if entry.kind != gix::objs::tree::EntryKind::Tree {
            continue;
        }

        // Each directory is an issue - read first file to get title/status
        let issue_path = format!("{}/{}", ISSUES_DIR, entry.filename);
        let comments =
            match get_git_tree(state, user_slug, project_slug, ISSUES_BRANCH, &issue_path).await {
                Ok(c) => c,
                Err(_) => continue,
            };

        let mut md_files: Vec<_> = comments
            .iter()
            .filter(|c| c.filename.ends_with(".md"))
            .map(|c| c.filename.clone())
            .collect();
        md_files.sort();

        if md_files.is_empty() {
            continue;
        }

        // Parse first comment for title
        let first_file = format!("{}/{}", issue_path, &md_files[0]);
        let content =
            match get_git_file(state, user_slug, project_slug, ISSUES_BRANCH, &first_file).await {
                Ok(c) => c,
                Err(_) => continue,
            };
        let content_str = String::from_utf8_lossy(&content.data);

        if let Ok((frontmatter, _)) = parse_frontmatter(&content_str) {
            let title = frontmatter
                .title
                .unwrap_or_else(|| "(No title)".to_string());

            // Determine current status by scanning all comments
            let mut status = IssueStatus::Open;
            for file in &md_files {
                let path = format!("{}/{}", issue_path, file);
                if let Ok(c) =
                    get_git_file(state, user_slug, project_slug, ISSUES_BRANCH, &path).await
                    && let Ok((fm, _)) = parse_frontmatter(&String::from_utf8_lossy(&c.data))
                    && let Some(s) = fm.status
                {
                    status = s;
                }
            }

            summaries.push(IssueSummary {
                dir_name: entry.filename.clone(),
                title,
                created_at: frontmatter.date,
                author: frontmatter.author,
                status,
                comment_count: md_files.len().saturating_sub(1),
            });
        }
    }

    // Sort by created_at descending (newest first)
    summaries.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(summaries)
}

/// Get a single issue with all comments
pub async fn get_issue(
    state: &GlobalState,
    user_slug: &str,
    project_slug: &str,
    issue_dir_name: &str,
) -> Result<Issue> {
    let issue_path = format!("{}/{}", ISSUES_DIR, issue_dir_name);

    let tree = get_git_tree(state, user_slug, project_slug, ISSUES_BRANCH, &issue_path).await?;

    let mut md_files: Vec<_> = tree
        .iter()
        .filter(|e| e.filename.ends_with(".md"))
        .map(|e| e.filename.clone())
        .collect();
    md_files.sort();

    if md_files.is_empty() {
        return Err(anyhow!("Issue has no comments"));
    }

    let mut comments = vec![];
    let mut title = String::new();

    for (i, filename) in md_files.iter().enumerate() {
        let path = format!("{}/{}", issue_path, filename);
        let content = get_git_file(state, user_slug, project_slug, ISSUES_BRANCH, &path).await?;
        let content_str = String::from_utf8_lossy(&content.data);

        let (frontmatter, body) = parse_frontmatter(&content_str)?;

        // First comment provides issue title
        if i == 0 {
            title = frontmatter
                .title
                .clone()
                .unwrap_or_else(|| "(No title)".to_string());
        }

        // Render markdown to HTML
        let html =
            markdown::to_html_with_options(&body, &markdown::Options::gfm()).unwrap_or_default();
        let html = ammonia::clean(&html);

        comments.push(IssueComment {
            date: frontmatter.date,
            author: frontmatter.author,
            content_html: html,
            status_change: frontmatter.status,
        });
    }

    let status = Issue::compute_status(&comments);

    Ok(Issue {
        dir_name: issue_dir_name.to_string(),
        title,
        status,
        comments,
    })
}

/// Create a new issue
pub async fn create_issue(
    state: &GlobalState,
    user: &User,
    project: &Project,
    title: &str,
    content: &str,
) -> Result<String> {
    // Ensure issues branch exists
    ensure_issues_branch(state, user, project).await?;

    // Get existing issue directories to ensure unique name
    let existing_dirs = if let Ok(tree) = get_git_tree(
        state,
        &project.owner,
        &project.slug,
        ISSUES_BRANCH,
        ISSUES_DIR,
    )
    .await
    {
        tree.iter().map(|e| e.filename.clone()).collect()
    } else {
        vec![]
    };

    let dir_name = generate_issue_dir_name(title, &existing_dirs);
    let filename = generate_comment_filename(&user.slug, &[]);

    let now = OffsetDateTime::now_utc();
    let formatted_date = now
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| anyhow!("Failed to format date: {}", e))?;

    let file_content = format!(
        "---\ndate: {}\nauthor: {}\nemail: {}\ntitle: {}\n---\n\n{}",
        formatted_date,
        user.slug,
        user.email,
        title,
        content.trim()
    );

    let file_path = format!("{}/{}/{}", ISSUES_DIR, dir_name, filename);

    add_file_to_branch(CommitParams {
        state,
        user_name: &project.owner,
        project_slug: &project.slug,
        branch_name: ISSUES_BRANCH,
        file_path: &file_path,
        file_content: &file_content,
        commit_message: &format!("Create issue: {}", title),
        author_name: &user.name,
        author_email: &user.email,
    })
    .await?;

    Ok(dir_name)
}

/// Add a comment to an existing issue
pub async fn add_comment(
    state: &GlobalState,
    user: &User,
    project: &Project,
    issue_dir_name: &str,
    content: &str,
    status: Option<IssueStatus>,
) -> Result<()> {
    let issue_path = format!("{}/{}", ISSUES_DIR, issue_dir_name);

    // Get existing filenames to ensure unique
    let existing = if let Ok(tree) = get_git_tree(
        state,
        &project.owner,
        &project.slug,
        ISSUES_BRANCH,
        &issue_path,
    )
    .await
    {
        tree.iter().map(|e| e.filename.clone()).collect()
    } else {
        vec![]
    };

    let filename = generate_comment_filename(&user.slug, &existing);

    let now = OffsetDateTime::now_utc();
    let formatted_date = now
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| anyhow!("Failed to format date: {}", e))?;

    let mut frontmatter = format!(
        "---\ndate: {}\nauthor: {}\nemail: {}",
        formatted_date, user.slug, user.email,
    );

    if let Some(s) = status {
        frontmatter.push_str(&format!("\nstatus: {}", s.as_str()));
    }

    frontmatter.push_str("\n---\n\n");

    let file_content = format!("{}{}", frontmatter, content.trim());
    let file_path = format!("{}/{}", issue_path, filename);

    let commit_msg = if let Some(s) = status {
        format!("Update issue status to {}", s.as_str())
    } else {
        "Add comment".to_string()
    };

    add_file_to_branch(CommitParams {
        state,
        user_name: &project.owner,
        project_slug: &project.slug,
        branch_name: ISSUES_BRANCH,
        file_path: &file_path,
        file_content: &file_content,
        commit_message: &commit_msg,
        author_name: &user.name,
        author_email: &user.email,
    })
    .await?;

    Ok(())
}

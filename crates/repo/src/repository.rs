use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use anyhow::{Result, anyhow};
use gix::bstr::BString;
use gix::{Commit, Repository};
use smallvec::smallvec;
use time::OffsetDateTime;
use tokio::fs;

use crate::entry_kind::EntryKind;
use crate::error::RepoError;
use crate::types::{
    AuthorResolver, CommitParams, GitCommitInfo, GitRefInfo, GitSummary, GitTreeEntry,
};

/// Special project slugs that are allowed despite starting with a period
const SPECIAL_PROJECT_SLUGS: &[&str] = &[".profile"];

/// Simple slug validation for path safety
fn validate_slug(value: &str) -> Result<(), &'static str> {
    if value.len() < 3 {
        return Err("Value must be at least 3 characters.");
    }

    // Allow special slugs like .profile
    if SPECIAL_PROJECT_SLUGS.contains(&value) {
        return Ok(());
    }

    if value.starts_with('.') {
        return Err("Value cannot start with a period.");
    }

    if value
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '-' | '_' | '.'))
    {
        Ok(())
    } else {
        Err("Only lowercase letters, numbers, dashes, underscores, and periods are allowed.")
    }
}

fn ensure_safe_component(value: &str) -> Result<(), RepoError> {
    if value.is_empty() {
        return Err(RepoError::InvalidPath("empty path component".into()));
    }

    if let Err(msg) = validate_slug(value) {
        return Err(RepoError::InvalidPath(msg.into()));
    }

    Ok(())
}

fn get_git_repo(git_root: &Path, user_name: &str, project_slug: &str) -> Option<Repository> {
    let path = git_root.join(user_name).join(project_slug);
    match gix::open(path) {
        Ok(repo) => Some(repo),
        Err(e) => {
            eprintln!("{e}");
            None
        }
    }
}

fn configure_bare_repo(path: &Path) -> Result<(), RepoError> {
    std::fs::create_dir_all(path.join("hooks-disabled"))?;

    let config_values = [
        ("receive.fsckObjects", "true"),
        ("transfer.fsckObjects", "true"),
        ("fetch.fsckObjects", "true"),
        ("core.hooksPath", "hooks-disabled"),
    ];

    for (key, value) in config_values {
        let status = Command::new("git")
            .arg("-C")
            .arg(path)
            .arg("config")
            .arg("--local")
            .arg(key)
            .arg(value)
            .status()?;

        if !status.success() {
            return Err(RepoError::Git(format!("git config failed for {key}")));
        }
    }

    Ok(())
}

fn ensure_safe_branch_name(branch: &str) -> Result<(), RepoError> {
    if branch.trim() != branch || branch.contains('\n') || branch.contains('\r') {
        return Err(RepoError::InvalidPath("invalid branch name".into()));
    }

    let status = Command::new("git")
        .arg("check-ref-format")
        .arg("--branch")
        .arg(branch)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(RepoError::InvalidPath("invalid branch name".into()))
    }
}

/// Create a new bare git repository
pub async fn create_bare_repo(git_root: &Path, user: &str, project: &str) -> Result<(), RepoError> {
    ensure_safe_component(user)?;
    ensure_safe_component(project)?;

    let path = git_root.join(user);
    fs::create_dir_all(&path).await?;

    let path = path.join(project);

    tokio::task::spawn_blocking(move || {
        gix::init_bare(&path).map_err(|e| RepoError::Git(e.to_string()))?;
        configure_bare_repo(&path)?;
        Ok::<_, RepoError>(())
    })
    .await
    .map_err(|e| RepoError::TaskJoin(e.to_string()))??;

    Ok(())
}

/// Set the HEAD reference for a repository
pub async fn set_git_head(
    git_root: &Path,
    user_name: &str,
    project_slug: &str,
    head_branch: &str,
) -> Result<()> {
    ensure_safe_branch_name(head_branch)?;
    let contents = format!("ref: refs/heads/{}\n", head_branch);
    let path = git_root.join(user_name).join(project_slug).join("HEAD");
    fs::write(path, contents).await?;
    Ok(())
}

/// Get a summary of a repository's branches and tags
pub async fn get_git_summary(
    git_root: &Path,
    user_name: &str,
    project_slug: &str,
) -> Option<GitSummary> {
    let git_root = git_root.to_path_buf();
    let user_name = user_name.to_string();
    let project_slug = project_slug.to_string();

    let Ok(res) = tokio::task::spawn_blocking(move || {
        // Check repo exists
        let _ = get_git_repo(&git_root, &user_name, &project_slug)?;
        Some(capture_git_summary(&git_root, &user_name, &project_slug))
    })
    .await
    else {
        return None;
    };
    res
}

/// Capture the current state of a repository's refs (sync, for use in spawn_blocking)
pub fn capture_git_summary(git_root: &Path, owner: &str, project: &str) -> GitSummary {
    let path = git_root.join(owner).join(project);
    let mut branches = HashMap::new();
    let mut tags = HashMap::new();

    if let Ok(repo) = gix::open(&path)
        && let Ok(references) = repo.references()
    {
        if let Ok(iter) = references.prefixed("refs/heads/") {
            for mut r in iter.flatten() {
                if let Ok(commit) = r.peel_to_commit() {
                    branches.insert(r.name().shorten().to_string(), commit.id().to_string());
                }
            }
        }
        if let Ok(iter) = references.prefixed("refs/tags/") {
            for mut r in iter.flatten() {
                if let Ok(commit) = r.peel_to_commit() {
                    tags.insert(r.name().shorten().to_string(), commit.id().to_string());
                }
            }
        }
    }

    GitSummary::new(owner.to_string(), project.to_string(), branches, tags)
}

/// Get information about a git reference (branch/tag) with commits
pub async fn get_git_info<R: AuthorResolver>(
    git_root: &Path,
    user_name: &str,
    project_slug: &str,
    branch: &str,
    max_commits: i32,
    offset: i32,
    author_resolver: &R,
) -> Option<GitRefInfo> {
    if max_commits < 1 {
        return None;
    }

    let git_root = git_root.to_path_buf();
    let user_name = user_name.to_string();
    let project_slug = project_slug.to_string();
    let branch = branch.to_string();

    // We need to capture author resolution results in the blocking task
    // For now, collect emails and resolve after
    let Ok(res) = tokio::task::spawn_blocking(move || {
        let mut repo = get_git_repo(&git_root, &user_name, &project_slug)?;
        repo.object_cache_size(Some(4096 * 4096));

        // Try to resolve as reference first, fall back to rev_parse for commit hashes
        let (commit, ref_name) = if let Ok(mut reference) = repo.find_reference(&branch) {
            let commit = reference.peel_to_commit().ok()?;
            let name = reference.name().shorten().to_string();
            (commit, name)
        } else {
            // Try to parse as revision (commit hash, tag, etc.)
            let object = repo.rev_parse_single(branch.as_str()).ok()?;
            let commit = object.object().ok()?.try_into_commit().ok()?;
            (commit, branch.clone())
        };

        let commit_count: i32 = commit
            .ancestors()
            .all()
            .ok()?
            .count()
            .try_into()
            .unwrap_or_default();

        let walk = commit.ancestors().all().ok()?;
        let commits: Vec<CommitData> = walk
            .skip(offset.try_into().unwrap_or_default())
            .take(max_commits.try_into().unwrap_or_default())
            .flatten()
            .flat_map(|c| c.object().map(CommitData::from_commit))
            .collect();

        Some((ref_name, commit_count, commits))
    })
    .await
    else {
        return None;
    };

    // Now resolve author emails outside the blocking task
    let (ref_name, commit_count, commit_data) = res?;
    let commits = commit_data
        .into_iter()
        .map(|cd| {
            let author_user_slug = cd
                .author_email
                .as_ref()
                .and_then(|email| author_resolver.resolve_email(email));
            GitCommitInfo {
                id: cd.id,
                author: cd.author,
                author_user_slug,
                message: cd.message,
                time: cd.time,
            }
        })
        .collect();

    Some(GitRefInfo {
        branch_name: ref_name,
        commit_count,
        commits,
    })
}

/// Intermediate struct to hold commit data before author resolution
struct CommitData {
    id: String,
    author: String,
    author_email: Option<String>,
    message: String,
    time: OffsetDateTime,
}

impl CommitData {
    fn from_commit(commit: Commit) -> Self {
        let commit_id = commit.id().shorten_or_id().to_string();
        let author_ref = commit.author().ok();
        let commit_author = author_ref
            .as_ref()
            .map(|a| format!("{}", a.name))
            .unwrap_or_default();
        let author_email = author_ref.as_ref().map(|a| a.email.to_string());
        let commit_message = commit
            .message()
            .map(|m| m.summary().to_string())
            .unwrap_or_default();
        let gix_time = commit.time().unwrap_or_default();

        // Convert gix::date::Time to time::OffsetDateTime
        let time = OffsetDateTime::from_unix_timestamp(gix_time.seconds)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH);

        Self {
            id: commit_id,
            author: commit_author,
            author_email,
            message: commit_message,
            time,
        }
    }
}

/// Get the contents of a file from a repository
pub async fn get_git_file(
    git_root: &Path,
    user_name: &str,
    project_slug: &str,
    branch: &str,
    path: &str,
) -> Result<Vec<u8>> {
    let git_root = git_root.to_path_buf();
    let user_name = user_name.to_string();
    let project_slug = project_slug.to_string();
    let branch = branch.to_string();
    let path = path.to_string();

    let Ok(res) = tokio::task::spawn_blocking(move || {
        let repo = get_git_repo(&git_root, &user_name, &project_slug)
            .ok_or(anyhow!("Couldn't get Repository"))?;

        // Try to resolve as reference first, fall back to rev_parse for commit hashes
        let commit = if let Ok(mut reference) = repo.find_reference(&branch) {
            reference.peel_to_commit()?
        } else {
            // Try to parse as revision (commit hash, tag, etc.)
            let object = repo.rev_parse_single(branch.as_str())?;
            object.object()?.try_into_commit()?
        };

        let entry = commit
            .tree()?
            .lookup_entry_by_path(path)?
            .ok_or(anyhow!("Can't lookup entry"))?;
        let blob = entry.object()?.try_into_blob()?;

        Ok(blob.data.to_vec())
    })
    .await
    else {
        return Err(anyhow!("Error when getting git file"));
    };
    res
}

/// Get the entries in a directory from a repository
pub async fn get_git_tree(
    git_root: &Path,
    user_name: &str,
    project_slug: &str,
    branch: &str,
    path: &str,
) -> Result<Vec<GitTreeEntry>> {
    let git_root = git_root.to_path_buf();
    let user_name = user_name.to_string();
    let project_slug = project_slug.to_string();
    let branch = branch.to_string();
    let path = path.to_string();

    let Ok(res) = tokio::task::spawn_blocking(move || {
        let repo = get_git_repo(&git_root, &user_name, &project_slug)
            .ok_or(anyhow!("Couldn't get Repository"))?;

        // Try to resolve as reference first, fall back to rev_parse for commit hashes
        let commit = if let Ok(mut reference) = repo.find_reference(&branch) {
            reference.peel_to_commit()?
        } else {
            // Try to parse as revision (commit hash, tag, etc.)
            let object = repo.rev_parse_single(branch.as_str())?;
            object.object()?.try_into_commit()?
        };

        let tree = if path.is_empty() {
            commit.tree()?
        } else {
            let entry = commit
                .tree()?
                .peel_to_entry_by_path(path)?
                .ok_or(anyhow!("Can't lookup entry"))?;

            entry.object()?.into_tree()
        };

        let mut entries: Vec<GitTreeEntry> = tree
            .iter()
            .flatten()
            .map(|entry| {
                let filename = entry.filename().to_string();
                let kind = EntryKind::from_gix(entry.kind());
                GitTreeEntry { filename, kind }
            })
            .collect();
        entries.sort();

        Ok(entries)
    })
    .await
    else {
        return Err(anyhow!("Error when getting git tree"));
    };
    res
}

/// Check if a branch exists in a repository
pub async fn branch_exists(
    git_root: &Path,
    user_name: &str,
    project_slug: &str,
    branch: &str,
) -> bool {
    let git_root = git_root.to_path_buf();
    let user_name = user_name.to_string();
    let project_slug = project_slug.to_string();
    let branch = branch.to_string();

    let Ok(res) = tokio::task::spawn_blocking(move || {
        let repo = get_git_repo(&git_root, &user_name, &project_slug)?;
        let ref_name = format!("refs/heads/{}", branch);
        Some(repo.find_reference(&ref_name).is_ok())
    })
    .await
    else {
        return false;
    };
    res.unwrap_or(false)
}

/// Create an orphan branch with an initial commit containing one file
pub async fn create_orphan_branch(params: CommitParams<'_>) -> Result<()> {
    let git_root = params.git_root.to_path_buf();
    let user_name = params.user_name.to_string();
    let project_slug = params.project_slug.to_string();
    let branch_name = params.branch_name.to_string();
    let file_path = params.file_path.to_string();
    let file_content = params.file_content.to_string();
    let commit_message = params.commit_message.to_string();
    let author_name = params.author_name.to_string();
    let author_email = params.author_email.to_string();

    tokio::task::spawn_blocking(move || -> Result<()> {
        let repo = get_git_repo(&git_root, &user_name, &project_slug)
            .ok_or_else(|| anyhow!("Could not open repository"))?;

        // 1. Write blob
        let blob_id = repo.write_blob(file_content.as_bytes())?;

        // 2. Build tree using tree editor starting from empty tree
        let empty_tree = gix::ObjectId::empty_tree(repo.object_hash());
        let mut editor = repo.edit_tree(empty_tree)?;
        editor.upsert(&file_path, gix::objs::tree::EntryKind::Blob, blob_id)?;
        let tree_id = editor.write()?;

        // 3. Create signature
        let time = gix::date::Time::now_local_or_utc();
        let signature = gix::actor::Signature {
            name: BString::from(author_name),
            email: BString::from(author_email),
            time,
        };

        // 4. Create commit object (no parents = orphan)
        let commit = gix::objs::Commit {
            tree: tree_id.detach(),
            parents: smallvec![],
            author: signature.clone(),
            committer: signature,
            encoding: None,
            message: BString::from(commit_message.as_str()),
            extra_headers: vec![],
        };
        let commit_id = repo.write_object(&commit)?;

        // 5. Create reference
        let ref_name = format!("refs/heads/{}", branch_name);
        repo.reference(
            ref_name,
            commit_id,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "Create orphan branch",
        )?;

        Ok(())
    })
    .await
    .map_err(|e| anyhow!("Task join error: {}", e))??;

    Ok(())
}

/// Add a file to a branch and create a commit
pub async fn add_file_to_branch(params: CommitParams<'_>) -> Result<()> {
    let git_root = params.git_root.to_path_buf();
    let user_name = params.user_name.to_string();
    let project_slug = params.project_slug.to_string();
    let branch_name = params.branch_name.to_string();
    let file_path = params.file_path.to_string();
    let file_content = params.file_content.to_string();
    let commit_message = params.commit_message.to_string();
    let author_name = params.author_name.to_string();
    let author_email = params.author_email.to_string();

    tokio::task::spawn_blocking(move || -> Result<()> {
        let repo = get_git_repo(&git_root, &user_name, &project_slug)
            .ok_or_else(|| anyhow!("Could not open repository"))?;

        // 1. Get current branch tip and its tree
        let ref_name = format!("refs/heads/{}", branch_name);
        let mut reference = repo.find_reference(&ref_name)?;
        let parent_commit = reference.peel_to_commit()?;
        let tree_id = parent_commit.tree_id()?;

        // 2. Write new blob
        let blob_id = repo.write_blob(file_content.as_bytes())?;

        // 3. Edit tree to add/update file
        let mut editor = repo.edit_tree(tree_id)?;
        editor.upsert(&file_path, gix::objs::tree::EntryKind::Blob, blob_id)?;
        let new_tree_id = editor.write()?;

        // 4. Create signature
        let time = gix::date::Time::now_local_or_utc();
        let signature = gix::actor::Signature {
            name: BString::from(author_name),
            email: BString::from(author_email),
            time,
        };

        // 5. Create commit object with parent
        let commit = gix::objs::Commit {
            tree: new_tree_id.detach(),
            parents: smallvec![parent_commit.id().detach()],
            author: signature.clone(),
            committer: signature,
            encoding: None,
            message: BString::from(commit_message.as_str()),
            extra_headers: vec![],
        };
        let new_commit_id = repo.write_object(&commit)?;

        // 6. Update reference
        reference.set_target_id(new_commit_id, commit_message)?;

        Ok(())
    })
    .await
    .map_err(|e| anyhow!("Task join error: {}", e))??;

    Ok(())
}

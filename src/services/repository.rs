use anyhow::{Result, anyhow};
use gix::bstr::BString;
use gix::{Commit, ObjectDetached, Repository, date::Time, objs::tree::EntryKind};
use smallvec::smallvec;
use std::{
    io,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::fs;

use crate::{
    GlobalState, models::common::format_relative_time, services::validation::validate_slug,
};

fn ensure_safe_component(value: &str) -> io::Result<()> {
    if value.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid path component",
        ));
    }

    if let Err(msg) = validate_slug(value) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, msg));
    }

    Ok(())
}

pub async fn create_bare_repo(
    state: &GlobalState,
    user: String,
    project: String,
) -> Result<(), std::io::Error> {
    ensure_safe_component(&user)?;
    ensure_safe_component(&project)?;

    let path = state.config.git_root.join(&user);
    fs::create_dir_all(&path).await?;

    let path = path.join(project);

    // Use gix to create bare repo (faster and quieter than shelling out to git)
    tokio::task::spawn_blocking(move || {
        gix::init_bare(&path).map_err(|e| io::Error::other(e.to_string()))?;
        Ok::<_, io::Error>(())
    })
    .await
    .map_err(|e| io::Error::other(e.to_string()))??;

    Ok(())
}

fn get_git_repo(state: &GlobalState, user_name: &str, project_slug: &str) -> Option<Repository> {
    let path = state.config.git_root.join(user_name).join(project_slug);
    match gix::open(path) {
        Ok(repo) => Some(repo),
        Err(e) => {
            eprintln!("{e}");
            None
        }
    }
}

pub async fn get_git_summary(
    state: &GlobalState,
    user_name: &str,
    project_slug: &str,
) -> Option<GitSummary> {
    let state = state.clone();
    let user_name = user_name.to_string();
    let project_slug = project_slug.to_string();

    let Ok(res) = tokio::task::spawn_blocking(move || {
        let repo = get_git_repo(&state, &user_name, &project_slug)?;
        let mut tags = vec![];
        let mut branches = vec![];

        if let Ok(refs) = repo.references() {
            if let Ok(iter) = refs.prefixed("refs/tags/") {
                for r in iter.flatten() {
                    tags.push(r.name().shorten().to_string());
                }
            }

            if let Ok(iter) = refs.prefixed("refs/heads/") {
                for r in iter.flatten() {
                    branches.push(r.name().shorten().to_string());
                }
            }
        }

        Some(GitSummary { branches, tags })
    })
    .await
    else {
        return None;
    };
    res
}

pub async fn get_git_info(
    state: &GlobalState,
    user_name: &str,
    project_slug: &str,
    branch: &str,
    max_commits: i32,
    offset: i32,
) -> Option<GitRefInfo> {
    if max_commits < 1 {
        return None;
    }

    let state = state.clone();
    let user_name = user_name.to_string();
    let project_slug = project_slug.to_string();
    let branch = branch.to_string();

    let Ok(res) = tokio::task::spawn_blocking(move || {
        let mut repo = get_git_repo(&state, &user_name, &project_slug)?;
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
        let commits: Vec<GitCommitInfo> = walk
            .skip(offset.try_into().unwrap_or_default())
            .take(max_commits.try_into().unwrap_or_default())
            .flatten()
            .flat_map(|c| c.object().map(|o| o.into()))
            .collect();

        Some(GitRefInfo {
            branch_name: ref_name,
            commit_count,
            commits,
        })
    })
    .await
    else {
        return None;
    };
    res
}

impl From<Commit<'_>> for GitCommitInfo {
    fn from(commit: Commit) -> Self {
        let commit_id = commit.id().shorten_or_id().to_string();
        let commit_author = commit
            .author()
            .map(|a| format!("{}", a.name))
            .unwrap_or_default();
        let commit_message = commit
            .message()
            .map(|m| m.summary().to_string())
            .unwrap_or_default();
        let commit_time = commit.time().unwrap_or_default();

        Self {
            id: commit_id.to_string(),
            author: commit_author,
            message: commit_message,
            time: commit_time,
        }
    }
}

pub async fn get_git_file(
    state: &GlobalState,
    user_name: &str,
    project_slug: &str,
    branch: &str,
    path: &str,
) -> Result<ObjectDetached> {
    let state = state.clone();
    let user_name = user_name.to_string();
    let project_slug = project_slug.to_string();
    let branch = branch.to_string();
    let path = path.to_string();

    let Ok(res) = tokio::task::spawn_blocking(move || {
        let repo = get_git_repo(&state, &user_name, &project_slug)
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

        Ok(blob.detach())
    })
    .await
    else {
        return Err(anyhow!("Error when getting git file"));
    };
    res
}

pub async fn get_git_tree(
    state: &GlobalState,
    user_name: &str,
    project_slug: &str,
    branch: &str,
    path: &str,
) -> Result<Vec<GitTreeEntry>> {
    let state = state.clone();
    let user_name = user_name.to_string();
    let project_slug = project_slug.to_string();
    let branch = branch.to_string();
    let path = path.to_string();

    let Ok(res) = tokio::task::spawn_blocking(move || {
        let repo = get_git_repo(&state, &user_name, &project_slug)
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

        let mut tree = tree
            .iter()
            .flatten()
            .map(|entry| {
                let filename = entry.filename().to_string();
                let kind = entry.kind();

                GitTreeEntry { filename, kind }
            })
            .collect::<Vec<GitTreeEntry>>();
        tree.sort();

        Ok(tree)
    })
    .await
    else {
        return Err(anyhow!("Error when getting git file"));
    };
    res
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GitTreeEntry {
    pub filename: String,
    pub kind: EntryKind,
}

impl GitTreeEntry {
    pub fn full_path(&self, current_path: &str) -> String {
        if current_path.is_empty() {
            self.filename.clone()
        } else {
            format!("{}/{}", current_path, self.filename)
        }
    }

    pub fn uri_tree(
        &self,
        project_owner: &str,
        project_slug: &str,
        git_ref: &str,
        current_path: &str,
    ) -> String {
        let full_path = self.full_path(current_path);
        let encoded_ref = urlencoding::encode(git_ref);
        format!(
            "/~{}/{}/tree/{}/{}",
            project_owner, project_slug, encoded_ref, full_path
        )
    }

    pub fn uri_blob(
        &self,
        project_owner: &str,
        project_slug: &str,
        git_ref: &str,
        current_path: &str,
    ) -> String {
        let full_path = self.full_path(current_path);
        let encoded_ref = urlencoding::encode(git_ref);
        format!(
            "/~{}/{}/blob/{}/{}",
            project_owner, project_slug, encoded_ref, full_path
        )
    }
}

impl Ord for GitTreeEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Sort directories before files, then alphabetically by name
        match (self.kind, other.kind) {
            // Both are directories - sort alphabetically
            (EntryKind::Tree, EntryKind::Tree) => self.filename.cmp(&other.filename),
            // Self is directory, other is not - self comes first
            (EntryKind::Tree, _) => std::cmp::Ordering::Less,
            // Other is directory, self is not - other comes first
            (_, EntryKind::Tree) => std::cmp::Ordering::Greater,
            // Neither is directory - sort alphabetically
            _ => self.filename.cmp(&other.filename),
        }
    }
}

impl PartialOrd for GitTreeEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone)]
pub struct GitSummary {
    pub branches: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GitCommitInfo {
    pub id: String,
    pub author: String,
    pub message: String,
    pub time: Time,
}

#[derive(Debug, Clone)]
pub struct GitRefInfo {
    pub branch_name: String,
    pub commit_count: i32,
    pub commits: Vec<GitCommitInfo>,
}

impl GitCommitInfo {
    pub fn relative_time(&self) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Couldn't get relative time")
            .as_secs() as i64;

        let diff = now - self.time.seconds;
        format_relative_time(diff)
    }
}

/// Check if a branch exists in the repository
pub async fn branch_exists(
    state: &GlobalState,
    user_name: &str,
    project_slug: &str,
    branch: &str,
) -> bool {
    let state = state.clone();
    let user_name = user_name.to_string();
    let project_slug = project_slug.to_string();
    let branch = branch.to_string();

    let Ok(res) = tokio::task::spawn_blocking(move || {
        let repo = get_git_repo(&state, &user_name, &project_slug)?;
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
pub async fn create_orphan_branch(
    state: &GlobalState,
    user_name: &str,
    project_slug: &str,
    branch_name: &str,
    file_path: &str,
    file_content: &str,
    commit_message: &str,
    author_name: &str,
    author_email: &str,
) -> Result<()> {
    let state = state.clone();
    let user_name = user_name.to_string();
    let project_slug = project_slug.to_string();
    let branch_name = branch_name.to_string();
    let file_path = file_path.to_string();
    let file_content = file_content.to_string();
    let commit_message = commit_message.to_string();
    let author_name = author_name.to_string();
    let author_email = author_email.to_string();

    let res = tokio::task::spawn_blocking(move || -> Result<()> {
        let repo = get_git_repo(&state, &user_name, &project_slug)
            .ok_or_else(|| anyhow!("Could not open repository"))?;

        // 1. Write blob
        let blob_id = repo.write_blob(file_content.as_bytes())?;

        // 2. Build tree using tree editor starting from empty tree
        let empty_tree = gix::ObjectId::empty_tree(repo.object_hash());
        let mut editor = repo.edit_tree(empty_tree)?;
        editor.upsert(&file_path, EntryKind::Blob, blob_id)?;
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
            "Create issues branch",
        )?;

        Ok(())
    })
    .await
    .map_err(|e| anyhow!("Task join error: {}", e))??;

    Ok(res)
}

/// Add a file to a branch and create a commit
pub async fn add_file_to_branch(
    state: &GlobalState,
    user_name: &str,
    project_slug: &str,
    branch_name: &str,
    file_path: &str,
    file_content: &str,
    commit_message: &str,
    author_name: &str,
    author_email: &str,
) -> Result<()> {
    let state = state.clone();
    let user_name = user_name.to_string();
    let project_slug = project_slug.to_string();
    let branch_name = branch_name.to_string();
    let file_path = file_path.to_string();
    let file_content = file_content.to_string();
    let commit_message = commit_message.to_string();
    let author_name = author_name.to_string();
    let author_email = author_email.to_string();

    let res = tokio::task::spawn_blocking(move || -> Result<()> {
        let repo = get_git_repo(&state, &user_name, &project_slug)
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
        editor.upsert(&file_path, EntryKind::Blob, blob_id)?;
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

    Ok(res)
}

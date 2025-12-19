use anyhow::{Result, anyhow};
use gix::{Commit, ObjectDetached, Repository, date::Time, objs::tree::EntryKind};
use std::{
    io,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{fs, process::Command};

use crate::{GlobalState, services::validation::validate_slug};

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

    let path = state.config.git_root.join(user);
    fs::create_dir_all(&path).await?;

    let path = path.join(project);
    let status = Command::new("git")
        .arg("init")
        .arg("--bare")
        .arg(path)
        .kill_on_drop(true) // makes shutdowns cleaner
        .status()
        .await?;

    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other("git init --bare failed"))
    }
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
        format!(
            "/~{}/{}/tree/{}/{}",
            project_owner, project_slug, git_ref, full_path
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
        format!(
            "/~{}/{}/blob/{}/{}",
            project_owner, project_slug, git_ref, full_path
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
        if diff < 60 {
            return format!("{} second{} ago", diff, if diff != 1 { "s" } else { "" });
        }
        if diff < 3600 {
            let diff = diff / 60;
            return format!("{} minute{} ago", diff, if diff != 1 { "s" } else { "" });
        }
        if diff < 86400 {
            let diff = diff / 3600;
            return format!("{} hour{} ago", diff, if diff != 1 { "s" } else { "" });
        }
        if diff < 86400 * 30 {
            let diff = diff / 86400;
            return format!("{} day{} ago", diff, if diff != 1 { "s" } else { "" });
        }
        if diff < 86400 * 365 {
            let diff = diff / (86400 * 30);
            return format!("{} month{} ago", diff, if diff != 1 { "s" } else { "" });
        }
        let diff = diff / (86400 * 365);
        format!("{} year{} ago", diff, if diff != 1 { "s" } else { "" })
    }
}

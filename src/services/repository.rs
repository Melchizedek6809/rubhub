use anyhow::{Result, anyhow};
use gix::{ObjectDetached, Repository, date::Time};
use std::{
    io,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{fs, process::Command};

use crate::{services::validation::validate_slug, state::GlobalState};

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

fn get_git_repo(
    state: &GlobalState,
    user_name: &str,
    project_slug: &str,
) -> Option<Repository> {
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
    }).await else {
        return None;
    };
    res

}

pub async fn get_git_info(
    state: &GlobalState,
    user_name: &str,
    project_slug: &str,
    branch: &str,
) -> Option<GitCommitInfo> {
    let state = state.clone();
    let user_name = user_name.to_string();
    let project_slug = project_slug.to_string();
    let branch = branch.to_string();

    let Ok(res) = tokio::task::spawn_blocking(move || {
        let repo = get_git_repo(&state, &user_name, &project_slug)?;
        let mut reference = repo.find_reference(&branch).ok()?;
        let commit = reference.peel_to_commit().ok()?;
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

        Some(GitCommitInfo {
            branch_name: reference.name().shorten().to_string(),
            commit_id: commit_id.to_string(),
            commit_author,
            commit_message,
            commit_time,
        })
    }).await else {
        return None;
    };
    res
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
        let repo =
            get_git_repo(&state, &user_name, &project_slug).ok_or(anyhow!("Couldn't get Repository"))?;
        let mut reference = repo.find_reference(&branch)?;

        let commit = reference.peel_to_commit()?;
        let entry = commit
            .tree()?
            .lookup_entry_by_path(path)?
            .ok_or(anyhow!("Can't lookup entry"))?;
        let blob = entry.object()?.try_into_blob()?;

        Ok(blob.detach())
    }).await else {
        return Err(anyhow!("Error when getting git file"));
    };
    res
}

pub struct GitSummary {
    pub branches: Vec<String>,
    pub tags: Vec<String>,
}

pub struct GitCommitInfo {
    pub branch_name: String,
    pub commit_id: String,
    pub commit_author: String,
    pub commit_message: String,
    pub commit_time: Time,
}

impl GitCommitInfo {
    pub fn relative_time(&self) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Couldn't get relative time")
            .as_secs() as i64;

        let diff = now - self.commit_time.seconds;
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

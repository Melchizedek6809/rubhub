use anyhow::Result;
use gix::{Repository, date::Time};
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

pub fn get_git_repo(
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

pub fn get_git_summary(
    state: &GlobalState,
    user_name: &str,
    project_slug: &str,
) -> Option<GitSummary> {
    let repo = get_git_repo(state, user_name, project_slug)?;
    let mut tags = vec![];
    let mut branches = vec![];

    if let Ok(refs) = repo.references() {
        if let Ok(iter) = refs.prefixed("refs/tags/") {
            for r in iter {
                if let Ok(r) = r {
                    tags.push(r.name().shorten().to_string());
                }
            }
        }

        if let Ok(iter) = refs.prefixed("refs/heads/") {
            for r in iter {
                if let Ok(r) = r {
                    branches.push(r.name().shorten().to_string());
                }
            }
        }
    }

    Some(GitSummary { branches, tags })
}

pub fn get_git_info(
    state: &GlobalState,
    user_name: &str,
    project_slug: &str,
    name: &str,
) -> Option<GitCommitInfo> {
    let repo = get_git_repo(state, user_name, project_slug)?;
    let Ok(mut reference) = repo.find_reference(name) else {
        return None;
    };
    let Ok(commit) = reference.peel_to_commit() else {
        return None;
    };
    let commit_id = commit.id().shorten_or_id().to_string();
    let commit_author = commit
        .author()
        .map(|a| format!("{} <{}>", a.name, a.email))
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
            .unwrap()
            .as_secs() as i64;

        let diff = now - self.commit_time.seconds;
        if diff < 60 {
            return format!("{} seconds ago", diff);
        }
        if diff < 3600 {
            return format!("{} minutes ago", diff / 60);
        }
        if diff < 86400 {
            return format!("{} hours ago", diff / 3600);
        }
        if diff < 86400 * 30 {
            return format!("{} days ago", diff / 86400);
        }
        if diff < 86400 * 365 {
            return format!("{} months ago", diff / (86400 * 30));
        }
        format!("{} years ago", diff / (86400 * 365))
    }
}

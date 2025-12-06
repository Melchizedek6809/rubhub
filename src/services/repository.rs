use std::io;
use anyhow::Result;
use gix::Repository;
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

pub fn get_git_repo(state: &GlobalState, user_name: &str, project_slug: &str) -> Option<Repository> {
    let path = state.config.git_root.join(user_name).join(project_slug);
    match gix::open(path) {
        Ok(repo) => Some(repo),
        Err(e) => {
            eprintln!("{e}");
            None
        }
    }
}

pub fn get_git_summary(state: &GlobalState, user_name: &str, project_slug: &str) -> Option<GitSummary> {
    let repo = get_git_repo(state, user_name, project_slug)?;
    let Ok(names) = repo.references() else { return None };
    let Ok(names) = names.all() else { return None };

    let branches = names.flatten().map(|b| b.name().shorten().to_string()).collect::<Vec<String>>();

    Some(GitSummary { branches })
}

pub fn get_git_info(state: &GlobalState, user_name: &str, project_slug: &str, name: &str) -> Option<GitCommitInfo> {
    let repo = get_git_repo(state, user_name, project_slug)?;
    println!("get_git_info {name}");
    let Ok(reference) = repo.find_reference(name) else { return None };
    let Ok(commit_id) = reference.id().shorten() else { return None };

    Some(GitCommitInfo {
        branch_name: reference.name().shorten().to_string(),
        commit_id: commit_id.to_string(),
    })
}

pub struct GitSummary {
    pub branches: Vec<String>,
}

pub struct GitCommitInfo {
    pub branch_name: String,
    pub commit_id: String,
}
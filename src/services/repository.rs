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

pub fn get_git_branches(state: &GlobalState, user_name: &str, project_slug: &str) -> Option<Vec<GitBranch>> {
    let repo = get_git_repo(state, user_name, project_slug)?;
    let names = repo.branch_names();
    Some(names.iter()
        .map(|s| s.to_string())
        .map(|name| GitBranch {
            name
        })
        .collect::<Vec<GitBranch>>())
}

pub struct GitBranch {
    pub name: String,
}

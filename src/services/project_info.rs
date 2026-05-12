use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::{
    GlobalState, Project,
    services::repository::{
        CommitParams, add_file_to_branch, branch_exists, create_orphan_branch, get_git_file,
    },
};

pub const INFO_BRANCH: &str = "meta/info";
const INFO_FILE: &str = "README.md";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectLink {
    pub web_url: String,
    pub git_ssh_url: String,
    pub git_http_url: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub name: String,
    pub description: String,
    pub default_branch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical: Option<ProjectLink>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<ProjectLink>,
}

pub fn project_link(state: &GlobalState, project: &Project) -> ProjectLink {
    ProjectLink {
        web_url: state.uri(&project.uri()),
        git_ssh_url: project.ssh_clone_url(&state.config.ssh_public_host),
        git_http_url: project.http_clone_url(&state.config.base_url),
    }
}

pub fn parse_info_document(content: &str) -> Result<ProjectMetadata> {
    if !content.starts_with("---\n") {
        return Err(anyhow!("Missing project info frontmatter"));
    }

    let rest = &content[4..];
    let end = rest
        .find("\n---\n")
        .or_else(|| rest.find("\n---"))
        .ok_or_else(|| anyhow!("Invalid project info frontmatter"))?;

    let yaml = &rest[..end];
    Ok(serde_yaml::from_str(yaml)?)
}

pub fn render_info_document(metadata: &ProjectMetadata) -> Result<String> {
    let yaml = serde_yaml::to_string(metadata)?;
    let body = if metadata.description.trim().is_empty() {
        format!("# {}\n", metadata.name)
    } else {
        format!("# {}\n\n{}\n", metadata.name, metadata.description.trim())
    };

    Ok(format!("---\n{}---\n\n{}", yaml, body))
}

pub async fn load_project_metadata(
    state: &GlobalState,
    owner: &str,
    project: &str,
) -> Result<ProjectMetadata> {
    let content = get_git_file(state, owner, project, INFO_BRANCH, INFO_FILE).await?;
    let content = String::from_utf8(content)?;
    parse_info_document(&content)
}

pub async fn write_project_metadata(
    state: &GlobalState,
    owner: &str,
    project: &str,
    metadata: &ProjectMetadata,
) -> Result<()> {
    let content = render_info_document(metadata)?;
    let params = CommitParams {
        state,
        user_name: owner,
        project_slug: project,
        branch_name: INFO_BRANCH,
        file_path: INFO_FILE,
        file_content: &content,
        commit_message: "Update project info",
        author_name: "RubHub",
        author_email: "noreply@rubhub.net",
    };

    if branch_exists(state, owner, project, INFO_BRANCH).await {
        add_file_to_branch(params).await
    } else {
        create_orphan_branch(params).await
    }
}

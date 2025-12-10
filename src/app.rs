use askama::Template;
#[cfg(debug_assertions)]
use tokio::fs;

use crate::{
    entities::{AccessType, Project, User},
    services::repository::{GitRefInfo, GitSummary},
};

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    featured: &'a [ProjectSummary<'a>],
}

#[derive(Template)]
#[template(path = "contact.html")]
struct ContactTemplate;

#[derive(Template)]
#[template(path = "404.html")]
struct NotFoundTemplate;

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate<'a> {
    message: Option<&'a str>,
}

#[derive(Template)]
#[template(path = "user_settings.html")]
struct UserSettingsTemplate<'a> {
    user: &'a User,
    ssh_keys: &'a [String],
    message: Option<&'a str>,
}

#[derive(Template)]
#[template(path = "user.html")]
struct UserTemplate<'a> {
    user: &'a User,
    projects: &'a [ProjectSummary<'a>],
    is_owner: bool,
}

#[derive(Clone, Debug)]
pub struct ProjectSummary<'a> {
    pub name: &'a str,
    pub slug: &'a str,
    pub owner_slug: &'a str,
    pub owner_name: &'a str,
    pub description: &'a str,
}

impl<'a> ProjectSummary<'a> {
    pub fn uri(&self) -> String {
        format!("/~{}/{}", self.owner_slug, self.slug)
    }

    pub fn owner_uri(&self) -> String {
        format!("/~{}", self.owner_slug)
    }
}

#[derive(Template)]
#[template(path = "project_new.html")]
struct NewProjectTemplate<'a> {
    message: Option<&'a str>,
    public_access: &'a str,
}

#[derive(Template)]
#[template(path = "project.html")]
struct ProjectTemplate<'a> {
    owner: &'a User,
    project: &'a Project,
    access_level: AccessType,
    ssh_clone_url: String,
    selected_branch: String,
    summary: GitSummary,
    info: Option<GitRefInfo>,
    readme_html: Option<String>,
}

#[derive(Template)]
#[template(path = "project_commits.html")]
struct ProjectCommitsTemplate<'a> {
    owner: &'a User,
    project: &'a Project,
    selected_branch: String,
    access_level: AccessType,
    ssh_clone_url: String,
    summary: GitSummary,
    info: Option<GitRefInfo>,
    current_page: usize,
    page_count: usize,
}

#[derive(Template)]
#[template(path = "project_settings.html")]
struct ProjectSettingsTemplate<'a> {
    owner: &'a User,
    project: &'a Project,
    message: Option<&'a str>,
}

#[cfg(not(debug_assertions))]
const APP_THEME: &str = include_str!("../dist/app.html");

pub fn extract_html_parts(html: &str) -> (&str, &str) {
    let first_split = html
        .split_once("</head>")
        .expect("extract_html_parts first_split");
    let head = first_split
        .0
        .split_once("<head>")
        .expect("extract_html_parts head")
        .1;
    let rest = first_split
        .1
        .split_once("<body>")
        .expect("extract_html_parts rest")
        .1;
    let body = rest
        .split_once("</body>")
        .expect("extract_html_parts body")
        .0;

    (head, body)
}

async fn theme(head: &str, body: &str) -> String {
    #[cfg(not(debug_assertions))]
    let contents = APP_THEME;
    #[cfg(debug_assertions)]
    let contents = fs::read_to_string("dist/app.html").await.unwrap();

    let contents = contents.replace("<!--HEAD-->", head);
    contents.replace("<!--BODY-->", body)
}

pub async fn index(featured: &[ProjectSummary<'_>]) -> String {
    let contents = IndexTemplate { featured }.render().unwrap();
    let (head, body) = extract_html_parts(&contents);
    theme(head, body).await
}

pub async fn contact() -> String {
    let contents = ContactTemplate.render().unwrap();
    let (head, body) = extract_html_parts(&contents);
    theme(head, body).await
}

pub async fn not_found() -> String {
    let contents = NotFoundTemplate.render().unwrap();
    let (head, body) = extract_html_parts(&contents);
    theme(head, body).await
}

pub async fn login(message: Option<&str>) -> String {
    let contents = LoginTemplate { message }.render().unwrap();

    let parts = extract_html_parts(&contents);

    theme(parts.0, parts.1).await
}

pub async fn settings(user: User, ssh_keys: &[String], message: Option<&str>) -> String {
    let contents = UserSettingsTemplate {
        user: &user,
        ssh_keys,
        message,
    }
    .render()
    .unwrap();

    let parts = extract_html_parts(&contents);

    theme(parts.0, parts.1).await
}

pub async fn projects(user: &User, projects: &[ProjectSummary<'_>], is_owner: bool) -> String {
    let contents = UserTemplate {
        user,
        projects,
        is_owner,
    }
    .render()
    .unwrap();

    let parts = extract_html_parts(&contents);

    theme(parts.0, parts.1).await
}

pub async fn new_project(message: Option<&str>, public_access: AccessType) -> String {
    let contents = NewProjectTemplate {
        message,
        public_access: public_access.as_str(),
    }
    .render()
    .unwrap();

    let parts = extract_html_parts(&contents);

    theme(parts.0, parts.1).await
}

pub async fn project_with_access(
    owner: User,
    project: Project,
    access_level: AccessType,
    ssh_clone_url: String,
    summary: GitSummary,
    info: Option<GitRefInfo>,
    readme_html: Option<String>,
) -> String {
    let selected_branch = info
        .as_ref()
        .map(|i| i.branch_name.to_string())
        .unwrap_or_default();

    let contents = ProjectTemplate {
        owner: &owner,
        project: &project,
        access_level,
        ssh_clone_url,
        summary,
        info,
        selected_branch,
        readme_html,
    }
    .render()
    .unwrap();

    let parts = extract_html_parts(&contents);

    theme(parts.0, parts.1).await
}

pub async fn project_commits(
    owner: User,
    project: Project,
    access_level: AccessType,
    ssh_clone_url: String,
    summary: GitSummary,
    info: Option<GitRefInfo>,
    current_page: usize,
    page_count: usize,
) -> String {
    let selected_branch = info
        .as_ref()
        .map(|i| i.branch_name.to_string())
        .unwrap_or_default();

    let contents = ProjectCommitsTemplate {
        owner: &owner,
        project: &project,
        access_level,
        ssh_clone_url,
        summary,
        info,
        selected_branch,
        current_page,
        page_count,
    }
    .render()
    .unwrap();

    let parts = extract_html_parts(&contents);

    theme(parts.0, parts.1).await
}

pub async fn project_settings(owner: User, project: Project, message: Option<&str>) -> String {
    let contents = ProjectSettingsTemplate {
        owner: &owner,
        project: &project,
        message,
    }
    .render()
    .unwrap();

    let parts = extract_html_parts(&contents);

    theme(parts.0, parts.1).await
}

#[cfg(test)]
mod tests {
    use super::extract_html_parts;

    #[test]
    fn extract_html_parts_returns_head_and_body() {
        let html = "<html><head><title>Test</title></head><body><h1>Hi</h1></body></html>";
        let (head, body) = extract_html_parts(html);

        assert_eq!(head, "<title>Test</title>");
        assert_eq!(body, "<h1>Hi</h1>");
    }

    #[test]
    fn extract_html_parts_handles_whitespace() {
        let html =
            "<html><head>\n<meta charset=\"utf-8\">\n</head><body>\n<p>Test</p>\n</body></html>";
        let (head, body) = extract_html_parts(html);

        assert_eq!(head, "\n<meta charset=\"utf-8\">\n");
        assert_eq!(body, "\n<p>Test</p>\n");
    }
}

use anyhow::{Result, anyhow};
use reqwest::{Client, Response};

/// A test API client that wraps reqwest with cookie jar and provides
/// convenient methods for interacting with RubHub.
pub struct Api {
    client: Client,
    base_url: String,
}

impl Api {
    /// Create a new Api instance with the given base URL.
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .cookie_store(true)
            .build()
            .expect("Failed to create reqwest client");

        Self {
            client,
            base_url: base_url.to_string(),
        }
    }

    /// Make a GET request to the given path and return the response.
    pub async fn post(&self, path: &str) -> Result<Response> {
        let url = format!("{}{}", self.base_url, path);
        Ok(self.client.post(&url).send().await?.error_for_status()?)
    }

    /// Make a GET request to the given path and return the response.
    pub async fn get(&self, path: &str) -> Result<Response> {
        let url = format!("{}{}", self.base_url, path);
        Ok(self.client.get(&url).send().await?.error_for_status()?)
    }

    /// Make a GET request and return the response body as text.
    pub async fn get_text(&self, path: &str) -> Result<String> {
        Ok(self.get(path).await?.text().await?)
    }

    /// Assert that a GET request to the given path contains the expected text.
    pub async fn assert_contains(&self, path: &str, needle: &str) -> Result<()> {
        let body = self.get_text(path).await?;
        if body.contains(needle) {
            Ok(())
        } else {
            eprintln!("{body}");
            Err(anyhow!("{}{} is missing {:?}", self.base_url, path, needle))
        }
    }

    /// Register a new user account.
    pub async fn register(&self, username: &str, email: &str, password: &str) -> Result<Response> {
        let form = [
            ("username", username),
            ("email", email),
            ("password", password),
        ];

        Ok(self
            .client
            .post(format!("{}/registration", self.base_url))
            .form(&form)
            .send()
            .await?
            .error_for_status()?)
    }

    /// Login with the given credentials.
    pub async fn login(&self, username: &str, password: &str) -> Result<Response> {
        let form = [("username", username), ("password", password)];

        Ok(self
            .client
            .post(format!("{}/login", self.base_url))
            .form(&form)
            .send()
            .await?
            .error_for_status()?)
    }

    /// Logout the current user.
    pub async fn logout(&self) -> Result<Response> {
        self.post("/logout").await
    }

    /// Create a new project with the given name and description.
    pub async fn create_project(&self, name: &str, description: &str) -> Result<Response> {
        let form = [("name", name), ("description", description)];

        Ok(self
            .client
            .post(format!("{}/projects/new", self.base_url))
            .form(&form)
            .send()
            .await?
            .error_for_status()?)
    }

    /// Create a new project with custom public_access level.
    ///
    /// public_access should be one of: "none", "read", "write"
    pub async fn create_project_with_access(
        &self,
        name: &str,
        description: &str,
        public_access: &str,
    ) -> Result<Response> {
        let form = [
            ("name", name),
            ("description", description),
            ("public_access", public_access),
        ];

        Ok(self
            .client
            .post(format!("{}/projects/new", self.base_url))
            .form(&form)
            .send()
            .await?
            .error_for_status()?)
    }

    /// Update user settings including SSH keys.
    pub async fn update_settings(
        &self,
        name: &str,
        email: &str,
        website: &str,
        description: &str,
        default_main_branch: &str,
        ssh_keys: &str,
    ) -> Result<Response> {
        let form = [
            ("name", name),
            ("email", email),
            ("website", website),
            ("description", description),
            ("default_main_branch", default_main_branch),
            ("ssh_keys", ssh_keys),
        ];

        Ok(self
            .client
            .post(format!("{}/settings", self.base_url))
            .form(&form)
            .send()
            .await?
            .error_for_status()?)
    }

    /// Make a GET request without calling error_for_status (for testing error responses).
    pub async fn get_raw(&self, path: &str) -> Result<Response> {
        let url = format!("{}{}", self.base_url, path);
        Ok(self.client.get(&url).send().await?)
    }

    /// Update project settings.
    pub async fn update_project_settings(
        &self,
        owner: &str,
        project: &str,
        name: &str,
        description: &str,
        public_access: &str,
        main_branch: &str,
        website: &str,
    ) -> Result<Response> {
        let path = format!("/~{}/{}/settings", owner, project);

        let form = [
            ("name", name),
            ("description", description),
            ("public_access", public_access),
            ("main_branch", main_branch),
            ("website", website),
        ];

        Ok(self
            .client
            .post(format!("{}{}", self.base_url, path))
            .form(&form)
            .send()
            .await?
            .error_for_status()?)
    }

    /// Create a new issue in a project.
    pub async fn create_issue(
        &self,
        owner: &str,
        project: &str,
        title: &str,
        content: &str,
    ) -> Result<Response> {
        let path = format!("/~{}/{}/talk/new", owner, project);

        let form = [("title", title), ("content", content)];

        Ok(self
            .client
            .post(format!("{}{}", self.base_url, path))
            .form(&form)
            .send()
            .await?
            .error_for_status()?)
    }

    /// Add a comment to an issue, optionally changing its status.
    ///
    /// status can be: None, Some("open"), Some("completed"), Some("cancelled")
    pub async fn add_issue_comment(
        &self,
        owner: &str,
        project: &str,
        issue_dir: &str,
        content: &str,
        status: Option<&str>,
    ) -> Result<Response> {
        let view_path = format!("/~{}/{}/talk/{}", owner, project, issue_dir);

        let post_path = format!("{}/comment", view_path);
        let status_value = status.unwrap_or("");

        let form = [("content", content), ("status", status_value)];

        Ok(self
            .client
            .post(format!("{}{}", self.base_url, post_path))
            .form(&form)
            .send()
            .await?
            .error_for_status()?)
    }
}

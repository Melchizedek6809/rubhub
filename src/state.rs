use std::{
    env, fs,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
    time::Instant,
};

use anyhow::{Context, Result};
use rubhub_auth_store::AuthStore;
use tokio::sync::broadcast;

use crate::models::{ContentPage, RepoEvent};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub dir_root: PathBuf,
    pub git_root: PathBuf,
    pub session_root: PathBuf,
    pub http_bind_addr: SocketAddr,
    pub ssh_bind_addr: SocketAddr,
    pub ssh_public_host: String,
    pub ssh_public_host_is_set: bool,
    pub base_url: String,
    pub base_url_is_set: bool,
    pub reuse_port: bool,
    pub content_pages: Vec<ContentPage>,
    pub index_content: Option<ContentPage>,
    pub featured_projects: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        let dir_root = PathBuf::from("./data/");

        let git_root = dir_root.join("git");
        let session_root = dir_root.join("sessions");

        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let port = 3000;
        let http_bind_addr = SocketAddr::new(ip, port);

        let base_url = format!("http://{http_bind_addr}");

        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let port = 2222;
        let ssh_bind_addr = SocketAddr::new(ip, port);

        let ssh_public_host = if port == 22 {
            format!("{ip}")
        } else {
            format!("{ip}:{port}")
        };

        Self {
            dir_root,
            git_root,
            session_root,
            http_bind_addr,
            ssh_bind_addr,
            ssh_public_host,
            ssh_public_host_is_set: false,
            base_url,
            base_url_is_set: false,
            reuse_port: false,
            content_pages: vec![],
            index_content: None,
            featured_projects: vec![],
        }
    }
}

impl AppConfig {
    pub fn set_dir_root(mut self, dir_root: &str) -> Self {
        let dir_root = PathBuf::from(dir_root);

        let git_root = dir_root.join("git");
        let session_root = dir_root.join("sessions");

        self.dir_root = dir_root;
        self.git_root = git_root;
        self.session_root = session_root;

        self
    }

    pub fn set_http_bind_addr(mut self, bind_addr: &str) -> Result<Self> {
        let addr = bind_addr.parse::<SocketAddr>()?;

        self.http_bind_addr = addr;
        if !self.base_url_is_set {
            self.base_url = format!("http://{addr}");
        }

        Ok(self)
    }

    pub fn set_ssh_bind_addr(mut self, bind_addr: &str) -> Result<Self> {
        let addr = bind_addr.parse::<SocketAddr>()?;

        self.ssh_bind_addr = addr;
        if !self.ssh_public_host_is_set {
            self.ssh_public_host = if addr.port() == 22 {
                format!("{}", addr.ip())
            } else {
                format!("{}", addr)
            };
        }

        Ok(self)
    }

    pub fn set_base_url(mut self, base_url: &str) -> Self {
        self.base_url = base_url.to_string();
        self.base_url_is_set = true;
        self
    }

    pub fn set_ssh_public_host(mut self, host: &str) -> Self {
        self.ssh_public_host = host.to_string();
        self.ssh_public_host_is_set = true;
        self
    }

    pub fn set_reuse_port(mut self, reuse_port: bool) -> Self {
        self.reuse_port = reuse_port;
        self
    }

    pub fn set_content_pages(mut self, pages: Vec<ContentPage>) -> Self {
        self.content_pages = pages;
        self
    }

    pub fn set_index_content(mut self, page: Option<ContentPage>) -> Self {
        self.index_content = page;
        self
    }

    pub fn set_featured_projects(mut self, projects: Vec<String>) -> Self {
        self.featured_projects = projects;
        self
    }

    /// Update bind addresses and derived URLs after binding
    pub fn update_bound_addresses(mut self, http_addr: SocketAddr, ssh_addr: SocketAddr) -> Self {
        self.http_bind_addr = http_addr;
        if !self.base_url_is_set {
            self.base_url = format!("http://{}", http_addr);
        }

        self.ssh_bind_addr = ssh_addr;
        if !self.ssh_public_host_is_set {
            self.ssh_public_host = if ssh_addr.port() == 22 {
                format!("{}", ssh_addr.ip())
            } else {
                format!("{}", ssh_addr)
            };
        }

        self
    }

    pub fn load_env(self) -> Result<Self> {
        let config = self;

        let config = match env::var("DIR_ROOT") {
            Ok(dir_root) => config.set_dir_root(&dir_root),
            _ => config,
        };
        let config = match env::var("HTTP_BIND_ADDRESS") {
            Ok(addr) => config.set_http_bind_addr(&addr)?,
            _ => config,
        };
        let base_url_env = env::var("BASE_URL");
        let config = match &base_url_env {
            Ok(uri) if !uri.trim().is_empty() => config.set_base_url(uri),
            Ok(_) => {
                anyhow::bail!("BASE_URL cannot be empty");
            }
            Err(_) => config,
        };
        let config = match env::var("SSH_BIND_ADDRESS") {
            Ok(addr) => config.set_ssh_bind_addr(&addr)?,
            _ => config,
        };
        let config = match env::var("SSH_PUBLIC_HOST") {
            Ok(uri) => config.set_ssh_public_host(&uri),
            _ => config,
        };
        let config = match env::var("REUSE_PORT") {
            Ok(b) => config.set_reuse_port(b.to_lowercase() == "true"),
            _ => config,
        };
        let config = match env::var("SITE_CONTENT") {
            Ok(spec) => {
                let pages = parse_content_pages(&spec)
                    .context("Failed to parse SITE_CONTENT environment variable")?;
                config.set_content_pages(pages)
            }
            _ => config,
        };
        let config = match env::var("INDEX_CONTENT") {
            Ok(spec) => {
                let page = parse_index_content(&spec)
                    .context("Failed to parse INDEX_CONTENT environment variable")?;
                config.set_index_content(Some(page))
            }
            _ => config,
        };
        let config = match env::var("FEATURED_PROJECTS") {
            Ok(spec) => {
                let projects = parse_featured_projects(&spec)
                    .context("Failed to parse FEATURED_PROJECTS environment variable")?;
                config.set_featured_projects(projects)
            }
            _ => config,
        };

        #[cfg(not(debug_assertions))]
        validate_release_requirements(base_url_env.is_ok())?;

        Ok(config)
    }

    pub fn new() -> Result<Self> {
        let config: Self = Self::default();
        config.load_env()
    }

    pub fn build(self, process_start: Instant) -> Result<GlobalState> {
        GlobalState::new(self, process_start)
    }
}

fn parse_content_pages(spec: &str) -> Result<Vec<ContentPage>> {
    spec.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(ContentPage::parse)
        .collect()
}

fn parse_featured_projects(spec: &str) -> Result<Vec<String>> {
    spec.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            // Enforce ~ prefix for user projects
            if !s.starts_with('~') {
                anyhow::bail!(
                    "Featured project '{}' must start with ~ prefix (e.g., ~username/project)",
                    s
                );
            }
            // Strip the ~ prefix and store just the path
            Ok(s[1..].to_string())
        })
        .collect()
}

fn parse_index_content(spec: &str) -> Result<ContentPage> {
    let path = spec.trim();

    // Strip leading ~ if present
    let path = path.strip_prefix("~").unwrap_or(path);

    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() < 3 {
        anyhow::bail!(
            "Invalid path format: expected 'user/repo/file.md' or '~user/repo/file.md', got '{}'",
            spec
        );
    }

    let repo_owner = parts[0];
    let repo_slug = parts[1];
    let file_path = parts[2..].join("/");

    if file_path.is_empty() {
        anyhow::bail!("File path cannot be empty");
    }

    Ok(ContentPage {
        title: "Index".to_string(),
        slug: "index".to_string(),
        repo_owner: repo_owner.to_string(),
        repo_slug: repo_slug.to_string(),
        file_path,
    })
}

#[cfg_attr(debug_assertions, allow(dead_code))]
fn validate_release_requirements(base_url_set: bool) -> Result<()> {
    if !base_url_set {
        anyhow::bail!("BASE_URL must be set in release builds");
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct GlobalState {
    pub auth: Arc<AuthStore>,
    pub config: Arc<AppConfig>,
    pub process_start: Instant,
    pub event_tx: broadcast::Sender<RepoEvent>,
}

#[cfg(test)]
mod tests {
    use super::validate_release_requirements;

    #[test]
    fn validate_release_requirements_fails_without_base_url() {
        let err = validate_release_requirements(false).unwrap_err();
        assert_eq!(err.to_string(), "BASE_URL must be set in release builds");
    }

    #[test]
    fn validate_release_requirements_ok_when_present() {
        assert!(validate_release_requirements(true).is_ok());
    }
}

impl GlobalState {
    pub fn uri(&self, path: &str) -> String {
        format!("{}{}", self.config.base_url, path)
    }

    pub fn new(config: AppConfig, process_start: Instant) -> Result<Self> {
        fs::create_dir_all(&config.dir_root)?;
        fs::create_dir_all(&config.git_root)?;
        fs::create_dir_all(&config.session_root)?;

        let auth = AuthStore::new(config.dir_root.clone());
        let auth = Arc::new(auth);

        // Create broadcast channel for SSE events
        // 256 buffer - lagging receivers will drop old events
        let (event_tx, _) = broadcast::channel(256);

        let state = Self {
            auth,
            process_start,
            config: Arc::new(config),
            event_tx,
        };

        Ok(state)
    }

    /// Emit a repository event to all SSE listeners
    pub fn emit_event(&self, event: RepoEvent) {
        // Ignore send errors (no receivers is fine)
        let _ = self.event_tx.send(event);
    }
}

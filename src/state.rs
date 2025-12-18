use std::{
    env, fs,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
    time::Instant,
};

use anyhow::{Context, Result};

use crate::models::ContentPage;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub dir_root: PathBuf,
    pub git_root: PathBuf,
    pub session_root: PathBuf,
    pub http_bind_addr: SocketAddr,
    pub ssh_bind_addr: SocketAddr,
    pub ssh_public_host: String,
    pub base_url: String,
    pub reuse_port: bool,
    pub content_pages: Vec<ContentPage>,
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
            base_url,
            reuse_port: false,
            content_pages: vec![],
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
        self.base_url = format!("http://{addr}");

        Ok(self)
    }

    pub fn set_ssh_bind_addr(mut self, bind_addr: &str) -> Result<Self> {
        let addr = bind_addr.parse::<SocketAddr>()?;

        self.ssh_bind_addr = addr;
        self.ssh_public_host = if addr.port() == 22 {
            format!("{}", addr.ip())
        } else {
            format!("{}", addr)
        };

        Ok(self)
    }

    pub fn set_base_url(mut self, base_url: &str) -> Self {
        self.base_url = base_url.to_string();
        self
    }

    pub fn set_ssh_public_host(mut self, host: &str) -> Self {
        self.ssh_public_host = host.to_string();
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

    pub fn set_featured_projects(mut self, projects: Vec<String>) -> Self {
        self.featured_projects = projects;
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
        let config = match env::var("BASE_URL") {
            Ok(uri) => config.set_base_url(&uri),
            _ => config,
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
        let config = match env::var("FEATURED_PROJECTS") {
            Ok(spec) => {
                let projects = parse_featured_projects(&spec)
                    .context("Failed to parse FEATURED_PROJECTS environment variable")?;
                config.set_featured_projects(projects)
            }
            _ => config,
        };

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
        .map(|s| ContentPage::parse(s))
        .collect()
}

fn parse_featured_projects(spec: &str) -> Result<Vec<String>> {
    spec.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            // Enforce ~ prefix for user projects
            if !s.starts_with('~') {
                anyhow::bail!("Featured project '{}' must start with ~ prefix (e.g., ~username/project)", s);
            }
            // Strip the ~ prefix and store just the path
            Ok(s[1..].to_string())
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct GlobalState {
    pub config: Arc<AppConfig>,
    pub process_start: Instant,
}

impl GlobalState {
    pub fn uri(&self, path: &str) -> String {
        format!("{}{}", self.config.base_url, path)
    }

    pub fn new(config: AppConfig, process_start: Instant) -> Result<Self> {
        fs::create_dir_all(&config.git_root)?;
        fs::create_dir_all(&config.session_root)?;

        let state = Self {
            process_start,
            config: Arc::new(config),
        };

        Ok(state)
    }
}

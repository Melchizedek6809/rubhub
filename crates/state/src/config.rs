use std::{
    env, fs,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use anyhow::{Context, Result};
use serde::Deserialize;

use super::ContentPage;

/// Expands ~ at the start of a path to the user's home directory
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    } else if path == "~"
        && let Some(home) = dirs::home_dir()
    {
        return home;
    }
    PathBuf::from(path)
}

/// Searches for a config file in standard locations
pub fn find_config_file() -> Option<PathBuf> {
    // Check RUBHUB_CONFIG env var first
    if let Ok(path) = env::var("RUBHUB_CONFIG") {
        let path = expand_tilde(&path);
        if path.exists() {
            return Some(path);
        }
    }

    // Search paths in order of preference
    let home = dirs::home_dir()?;
    let paths = [
        home.join(".rubhub/config.toml"),
        home.join(".config/rubhub/config.toml"),
        PathBuf::from("/usr/local/etc/rubhub/config.toml"),
        PathBuf::from("/etc/rubhub/config.toml"),
    ];

    paths.into_iter().find(|p| p.exists())
}

/// TOML configuration file structure
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ConfigFile {
    dir_root: Option<String>,
    http_bind_address: Option<String>,
    ssh_bind_address: Option<String>,
    base_url: Option<String>,
    ssh_public_host: Option<String>,
    reuse_port: Option<bool>,
    site_content: Option<Vec<String>>,
    index_content: Option<String>,
    featured_projects: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub dir_root: PathBuf,
    pub git_root: PathBuf,
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
        let dir_root = dirs::home_dir()
            .map(|h| h.join(".rubhub"))
            .unwrap_or_else(|| PathBuf::from("./.rubhub"));

        let git_root = dir_root.join("git");
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
        self.dir_root = dir_root;
        self.git_root = git_root;

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

    /// Load configuration from TOML file if one exists
    pub fn load_toml(self) -> Result<Self> {
        let config_path = match find_config_file() {
            Some(path) => path,
            None => return Ok(self), // No config file found, use defaults
        };

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

        let file_config: ConfigFile = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;

        let mut config = self;

        if let Some(dir_root) = file_config.dir_root {
            config = config.set_dir_root(&expand_tilde(&dir_root).to_string_lossy());
        }
        if let Some(addr) = file_config.http_bind_address {
            config = config.set_http_bind_addr(&addr)?;
        }
        if let Some(addr) = file_config.ssh_bind_address {
            config = config.set_ssh_bind_addr(&addr)?;
        }
        if let Some(url) = file_config.base_url {
            if url.trim().is_empty() {
                anyhow::bail!("base_url cannot be empty in config file");
            }
            config = config.set_base_url(&url);
        }
        if let Some(host) = file_config.ssh_public_host {
            config = config.set_ssh_public_host(&host);
        }
        if let Some(reuse_port) = file_config.reuse_port {
            config = config.set_reuse_port(reuse_port);
        }
        if let Some(content_list) = file_config.site_content {
            let pages: Vec<ContentPage> = content_list
                .iter()
                .filter_map(|spec| ContentPage::parse(spec).ok())
                .collect();
            config = config.set_content_pages(pages);
        }
        if let Some(spec) = file_config.index_content {
            if let Ok(page) = ContentPage::parse_index(&spec) {
                config = config.set_index_content(Some(page));
            }
        }
        if let Some(projects_list) = file_config.featured_projects {
            let projects = parse_featured_projects(&projects_list.join(","))
                .context("Failed to parse featured_projects in config file")?;
            config = config.set_featured_projects(projects);
        }

        Ok(config)
    }

    /// Load configuration from environment variables (overrides TOML)
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
                let pages: Vec<ContentPage> = spec
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .filter_map(|s| ContentPage::parse(s).ok())
                    .collect();
                config.set_content_pages(pages)
            }
            _ => config,
        };
        let config = match env::var("INDEX_CONTENT") {
            Ok(spec) => {
                if let Ok(page) = ContentPage::parse_index(&spec) {
                    config.set_index_content(Some(page))
                } else {
                    config
                }
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
        validate_release_requirements(config.base_url_is_set)?;

        Ok(config)
    }

    pub fn new() -> Result<Self> {
        let config = Self::default();
        let config = config.load_toml()?; // Load from config file first
        config.load_env() // Then apply env var overrides
    }
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

#[cfg_attr(debug_assertions, allow(dead_code))]
fn validate_release_requirements(base_url_set: bool) -> Result<()> {
    if !base_url_set {
        anyhow::bail!("BASE_URL must be set in release builds");
    }
    Ok(())
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

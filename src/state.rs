use std::{env, fs, net::SocketAddr, path::PathBuf, sync::Arc, time::Instant};

use anyhow::Result;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub git_root: PathBuf,
    pub session_root: PathBuf,
    pub http_bind_addr: SocketAddr,
    pub ssh_bind_addr: SocketAddr,
    pub ssh_public_host: String,
    pub base_url: String,
}

impl AppConfig {
    pub fn set_dir_root(mut self, dir_root: &str) -> Self {
        let dir_root = PathBuf::from(dir_root);

        let git_root = dir_root.join("git");
        let session_root = dir_root.join("sessions");

        self.git_root = git_root;
        self.session_root = session_root;

        self
    }

    pub fn new() -> Result<Self> {
        let dir_root = env::var("DIR_ROOT").unwrap_or_else(|_| "./data/".to_owned());
        let dir_root = PathBuf::from(dir_root);

        let git_root = dir_root.join("git");
        let session_root = dir_root.join("sessions");

        let bind_addr = env::var("BIND_ADDR").ok();
        let http_bind_addr = if let Some(addr) = bind_addr {
            addr.parse::<SocketAddr>()?
        } else {
            let http_addr =
                env::var("HTTP_BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_owned());
            let http_port: u16 = env::var("HTTP_BIND_PORT")
                .ok()
                .and_then(|val| val.parse().ok())
                .unwrap_or(3000);
            format!("{http_addr}:{http_port}").parse::<SocketAddr>()?
        };

        let base_url = env::var("BASE_URL").unwrap_or_else(|_| format!("http://{http_bind_addr}"));

        let ssh_port: u16 = env::var("SSH_PORT")
            .ok()
            .and_then(|val| val.parse().ok())
            .unwrap_or(2222);
        let ssh_addr = env::var("SSH_BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_owned());
        let ssh_bind_addr = format!("{ssh_addr}:{ssh_port}").parse::<SocketAddr>()?;
        let ssh_public_host = env::var("SSH_PUBLIC_HOST")
            .or_else(|_| env::var("SSH_URL"))
            .unwrap_or_else(|_| {
                if ssh_port == 22 {
                    ssh_addr.clone()
                } else {
                    format!("{ssh_addr}:{ssh_port}")
                }
            });

        let state = Self {
            base_url,
            git_root,
            session_root,
            http_bind_addr,
            ssh_bind_addr,
            ssh_public_host,
        };

        Ok(state)
    }

    pub fn build(self, process_start: Instant) -> Result<GlobalState> {
        GlobalState::new(self, process_start)
    }
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

use std::{env, net::SocketAddr, path::PathBuf, sync::Arc, time::Instant};
use tokio::fs;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub git_root: PathBuf,
    pub session_root: PathBuf,
    pub http_bind_addr: SocketAddr,
    pub ssh_bind_addr: SocketAddr,
    pub ssh_public_host: String,
    pub base_url: String,
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

    pub async fn new(process_start: Instant) -> anyhow::Result<Self> {
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

        let (fsa, fsb) = tokio::join!(
            fs::create_dir_all(&git_root),
            fs::create_dir_all(&session_root),
        );
        fsa?;
        fsb?;

        let state = Self {
            process_start,
            config: Arc::new(AppConfig {
                base_url,
                git_root,
                session_root,
                http_bind_addr,
                ssh_bind_addr,
                ssh_public_host,
            }),
        };

        Ok(state)
    }
}

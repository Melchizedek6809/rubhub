use std::{env, net::SocketAddr, path::PathBuf};

use sea_orm::{Database, DatabaseConnection};
use tokio::fs;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub git_root: PathBuf,
    pub asset_root: PathBuf,
    pub http_bind_addr: SocketAddr,
    pub ssh_bind_addr: SocketAddr,
}

#[derive(Debug, Clone)]
pub struct GlobalState {
    pub db: DatabaseConnection,
    pub config: AppConfig,
}

impl GlobalState {
    pub async fn new() -> anyhow::Result<Self> {
        let db_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/rubhub".to_owned());

        let git_root = env::var("GIT_ROOT").unwrap_or_else(|_| "./data/git".to_owned());
        let git_root = PathBuf::from(git_root);

        let asset_root = env::var("ASSET_ROOT").unwrap_or_else(|_| "./data/assets".to_owned());
        let asset_root = PathBuf::from(asset_root);

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

        let ssh_port: u16 = env::var("SSH_PORT")
            .ok()
            .and_then(|val| val.parse().ok())
            .unwrap_or(2222);
        let ssh_addr = env::var("SSH_BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_owned());
        let ssh_bind_addr = format!("{ssh_addr}:{ssh_port}").parse::<SocketAddr>()?;

        let db = Database::connect(&db_url).await?;
        fs::create_dir_all(&git_root).await?;

        let state = Self {
            db,
            config: AppConfig {
                git_root,
                asset_root,
                http_bind_addr,
                ssh_bind_addr,
            },
        };

        Ok(state)
    }
}

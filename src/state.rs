use std::net::SocketAddr;
use std::path::PathBuf;

use sea_orm::{Database, DatabaseConnection};
use tokio::fs;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub git_root: PathBuf,
    pub asset_root: PathBuf,
    pub bind_addr: SocketAddr,
}

#[derive(Debug, Clone)]
pub struct GlobalState {
    pub db: DatabaseConnection,
    pub config: AppConfig,
}

impl GlobalState {
    pub async fn new() -> anyhow::Result<Self> {
        let db_url = std::env::var("DATABASE_URL")?;

        let git_root = std::env::var("GIT_ROOT").unwrap_or_else(|_| "./data/git".to_owned());
        let git_root = PathBuf::from(git_root);

        let asset_root = std::env::var("ASSET_ROOT").unwrap_or_else(|_| "./data/assets".to_owned());
        let asset_root = PathBuf::from(asset_root);

        let bind_addr = std::env::var("BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:3000".to_owned())
            .parse::<SocketAddr>()?;

        let db = Database::connect(&db_url).await?;
        fs::create_dir_all(&git_root).await?;

        let state = Self {
            db,
            config: AppConfig {
                git_root,
                asset_root,
                bind_addr,
            },
        };

        Ok(state)
    }
}

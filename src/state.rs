use std::net::SocketAddr;
use std::path::PathBuf;

use sea_orm::{Database, DatabaseConnection};
use tokio::fs;

#[derive(Debug, Clone)]
pub struct AppConfig {
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
        let asset_root = std::env::var("ASSET_ROOT").unwrap_or_else(|_| "./data".to_owned());
        let asset_root = PathBuf::from(asset_root);
        let bind_addr = std::env::var("BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:3000".to_owned())
            .parse::<SocketAddr>()?;

        let db = Database::connect(&db_url).await?;
        fs::create_dir_all(&asset_root).await?;

        let state = Self {
            db,
            config: AppConfig {
                asset_root,
                bind_addr,
            },
        };

        Ok(state)
    }
}

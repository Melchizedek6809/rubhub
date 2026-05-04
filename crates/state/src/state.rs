use std::{fs, sync::Arc, time::Instant};

use anyhow::Result;
use rubhub_auth_store::AuthStore;
use tokio::sync::broadcast;

use super::{AppConfig, RepoEvent};

/// Global application state
#[derive(Debug, Clone)]
pub struct GlobalState {
    pub auth: Arc<AuthStore>,
    pub config: Arc<AppConfig>,
    pub process_start: Instant,
    pub event_tx: broadcast::Sender<RepoEvent>,
}

impl GlobalState {
    /// Build a full URI from a path using the configured base URL
    pub fn uri(&self, path: &str) -> String {
        format!("{}{}", self.config.base_url, path)
    }

    /// Create a new GlobalState instance
    pub fn new(config: AppConfig, process_start: Instant) -> Result<Self> {
        fs::create_dir_all(&config.dir_root)?;
        fs::create_dir_all(&config.git_root)?;

        let auth = AuthStore::new(config.dir_root.clone());
        let auth = Arc::new(auth);

        // Create broadcast channel for SSE events
        // 256 buffer - lagging receivers will drop old events
        let (event_tx, _) = broadcast::channel(256);

        Ok(Self {
            auth,
            process_start,
            config: Arc::new(config),
            event_tx,
        })
    }

    /// Emit a repository event to all SSE listeners
    pub fn emit_event(&self, event: RepoEvent) {
        // Ignore send errors (no receivers is fine)
        let _ = self.event_tx.send(event);
    }
}

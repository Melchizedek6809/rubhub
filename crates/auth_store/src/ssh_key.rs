use std::sync::mpsc::SendError;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{event::StoreEvent, AuthStore};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SshKey {
    pub public_key: String,
    pub key_type: String,
    pub label: String,
    pub user_slug: String,

    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl SshKey {
    /// Parse an SSH key from authorized_keys format: `<type> <key> [comment]`
    pub fn from_authorized_keys_line(line: &str, user_slug: String) -> Option<Self> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        let mut parts = line.splitn(3, |c: char| c.is_ascii_whitespace());

        let key_type = parts.next()?.to_string();
        let public_key = parts.next()?.to_string();
        let label = parts
            .next()
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        Some(Self {
            public_key,
            key_type,
            label,
            user_slug,
            created_at: OffsetDateTime::now_utc(),
        })
    }

    /// Format as authorized_keys line: `<type> <key> [comment]`
    pub fn to_authorized_keys_line(&self) -> String {
        if self.label.is_empty() {
            format!("{} {}", self.key_type, self.public_key)
        } else {
            format!("{} {} {}", self.key_type, self.public_key, self.label)
        }
    }

    pub fn save(self, store: &AuthStore) -> Result<(), SendError<StoreEvent>> {
        store.store_event(StoreEvent::SshKey(self))
    }

    pub fn delete(store: &AuthStore, public_key: String) -> Result<(), SendError<StoreEvent>> {
        store.store_event(StoreEvent::SshKeyDelete { public_key })
    }
}

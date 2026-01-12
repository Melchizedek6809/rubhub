use std::sync::mpsc::SendError;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{event::StoreEvent, AuthStore};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub session_id: Uuid,

    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
    pub user_slug: String,
}

impl Session {
    pub fn new(user_slug: String) -> Self {
        let expires_at = time::OffsetDateTime::now_utc() + time::Duration::days(30);

        Self {
            session_id: Uuid::now_v7(),
            expires_at,
            user_slug,
        }
    }

    pub fn save(self, store: &AuthStore) -> Result<(), SendError<StoreEvent>> {
        store.store_event(StoreEvent::Session(self))
    }
}

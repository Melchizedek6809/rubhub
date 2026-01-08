use crate::session::Session;
use crate::ssh_key::SshKey;
use crate::user::User;
use serde::{Serialize,Deserialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "T")]  // This is the key part!
pub enum StoreEvent {
    Quit,
    ReopenLog,
    User(User),
    UserDelete{ slug: String },
    Session(Session),
    SessionDelete{ session_id: Uuid },
    SshKey(SshKey),
    SshKeyDelete{ public_key: String },
}


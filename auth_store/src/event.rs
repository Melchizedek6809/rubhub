use crate::{Session, User};
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
}


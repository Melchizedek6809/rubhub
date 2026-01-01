use crate::{Session, User};
use serde::{Serialize,Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "T")]  // This is the key part!
pub enum StoreEvent {
    Quit,
    User(User),
    Session(Session),
}


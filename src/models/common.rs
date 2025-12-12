use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessType {
    None,
    Read,
    Write,
    Admin,
}

impl std::fmt::Display for AccessType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            AccessType::None => "None",
            AccessType::Read => "Read",
            AccessType::Write => "Write",
            AccessType::Admin => "Admin",
        };
        write!(f, "{label}")
    }
}

impl AccessType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccessType::None => "none",
            AccessType::Read => "read",
            AccessType::Write => "write",
            AccessType::Admin => "admin",
        }
    }
}

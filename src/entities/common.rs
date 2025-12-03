use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "access_type")]
pub enum AccessType {
    #[sea_orm(string_value = "none")]
    None,
    #[sea_orm(string_value = "read")]
    Read,
    #[sea_orm(string_value = "write")]
    Write,
    #[sea_orm(string_value = "admin")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "user_type")]
pub enum UserType {
    #[sea_orm(string_value = "normal")]
    Normal,
    #[sea_orm(string_value = "temporary")]
    Temporary,
}

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::common::UserType;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub created_at: Option<DateTimeWithTimeZone>,
    pub last_login: Option<DateTimeWithTimeZone>,
    pub user_type: UserType,
    pub email: String,
    pub name: String,
    pub password_hash: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::session::Entity")]
    Sessions,
    #[sea_orm(has_many = "super::project::Entity")]
    Projects,
    #[sea_orm(has_many = "super::access::Entity")]
    Accesses,
    #[sea_orm(has_many = "super::project_message::Entity")]
    ProjectMessages,
}

impl Related<super::session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Sessions.def()
    }
}

impl Related<super::project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Projects.def()
    }
}

impl Related<super::access::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Accesses.def()
    }
}

impl Related<super::project_message::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProjectMessages.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

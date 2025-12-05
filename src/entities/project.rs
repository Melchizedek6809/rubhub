use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::common::AccessType;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "projects")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub created_at: Option<DateTimeWithTimeZone>,
    pub owner: Uuid,
    pub public_access: AccessType,
    #[sea_orm(unique)]
    pub slug: String,
    pub name: String,
    pub description: String,

    pub default_branch: String,
    pub newest_commit_time: Option<DateTimeWithTimeZone>,
    pub newest_commit_hash: Option<String>

}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::Owner",
        to = "super::user::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    User,
    #[sea_orm(has_many = "super::access::Entity")]
    Accesses,
    #[sea_orm(has_many = "super::project_message::Entity")]
    ProjectMessages,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
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

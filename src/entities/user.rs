use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::common::UserType;
use time::OffsetDateTime;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub created_at: Option<OffsetDateTime>,
    pub last_login: Option<OffsetDateTime>,
    pub user_type: UserType,
    #[sea_orm(unique)]
    pub slug: String,
    pub name: String,
    #[sea_orm(unique)]
    pub email: String,
    pub description: String,
    pub website: String,

    pub default_main_branch: String,
    pub password_hash: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::project::Entity")]
    Projects,
}

impl Related<super::project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Projects.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

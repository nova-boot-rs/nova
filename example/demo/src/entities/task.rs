use sea_orm::entity::prelude::*;

// SeaORM entity used by `NovaSql::add_entity(...)` so the demo can
// auto-create the table from the model definition.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tasks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub tenant_id: String,
    pub title: String,
    pub payload: Option<String>,
    pub status: Option<String>,
    pub created_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

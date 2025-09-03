use sea_orm::entity::prelude::*;

// sessions table
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "sessions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: String,
    pub service_name: String,
    pub client_addr: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub container_id: Option<String>,
    pub bytes_transferred: i64,
    pub status: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl Related<self::interactions::Entity> for Entity {
    fn to() -> RelationDef {
        self::interactions::Relation::Session.def()
    }
}

impl Related<self::artifacts::Entity> for Entity {
    fn to() -> RelationDef {
        self::artifacts::Relation::Session.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub mod interactions {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "interactions")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub session_id: String,
        pub data: Vec<u8>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(belongs_to = "super::Entity", from = "Column::SessionId", to = "super::Column::Id")]
        Session,
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod artifacts {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "artifacts")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub session_id: String,
        pub json: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(belongs_to = "super::Entity", from = "Column::SessionId", to = "super::Column::Id")]
        Session,
    }

    impl ActiveModelBehavior for ActiveModel {}
}

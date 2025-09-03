//! SeaORM entity models used by the database storage backend.
//!
//! These structs map to the SQLite tables created by `database_storage`:
//! - `sessions` — top-level session metadata
//! - `interactions` — ordered chunks of raw interaction bytes per session
//! - `artifacts` — JSON-serialized `CaptureArtifacts` per session

use sea_orm::entity::prelude::*;

/// Sessions table entity model.
///
/// Stores session lifecycle, addressing and status as strings for portability.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "sessions")]
pub struct Model {
    /// UUID as string primary key
    #[sea_orm(primary_key)]
    pub id: String,
    /// Service name (e.g. "ssh")
    pub service_name: String,
    /// Client socket address string (IP:port)
    pub client_addr: String,
    /// RFC3339 start timestamp
    pub start_time: String,
    /// Optional RFC3339 end timestamp
    pub end_time: Option<String>,
    /// Optional container identifier
    pub container_id: Option<String>,
    /// Byte count as 64-bit integer
    pub bytes_transferred: i64,
    /// Session status as string enum
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

/// Interactions table entity models.
pub mod interactions {
    use sea_orm::entity::prelude::*;

    /// Ordered chunks of interaction data associated to a session.
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "interactions")]
    pub struct Model {
        /// Auto-increment row id
        #[sea_orm(primary_key)]
        pub id: i32,
        /// Foreign key to `sessions.id`
        pub session_id: String,
        /// Raw binary data chunk
        pub data: Vec<u8>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Belongs to a session
        #[sea_orm(
            belongs_to = "super::Entity",
            from = "Column::SessionId",
            to = "super::Column::Id"
        )]
        Session,
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// Artifacts table entity models.
pub mod artifacts {
    use sea_orm::entity::prelude::*;

    /// JSON-serialized `CaptureArtifacts` associated to a session.
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "artifacts")]
    pub struct Model {
        /// Primary key and FK to `sessions.id`
        #[sea_orm(primary_key)]
        pub session_id: String,
        /// Pretty/compact JSON payload
        pub json: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Belongs to a session
        #[sea_orm(
            belongs_to = "super::Entity",
            from = "Column::SessionId",
            to = "super::Column::Id"
        )]
        Session,
    }

    impl ActiveModelBehavior for ActiveModel {}
}

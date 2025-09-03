//! SQLite-backed storage implementation using SeaORM.
//!
//! This backend persists sessions, interactions, and capture artifacts to a
//! local SQLite database. It honors the `MIEL_DB_PATH` environment variable
//! to select the database file location, otherwise defaults to `./miel.sqlite3`.

use std::env;
use std::path::Path;

use chrono::{DateTime, Utc};
use log::{debug, error, info};
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{Expr, Func};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, Database, DatabaseConnection, DbBackend, EntityTrait,
    QueryFilter, QueryOrder, Set, Statement,
};
use uuid::Uuid;

use crate::error_handling::types::StorageError;
use crate::storage::db_entities as session;
use crate::storage::db_entities::artifacts as art;
use crate::storage::db_entities::interactions as inter;
use crate::storage::storage_trait::Storage;
use crate::storage::types::{CaptureArtifacts, Session, SessionFilter};

/// Storage backend that uses SQLite via SeaORM.
///
/// Construct with [`DatabaseStorage::new`] to respect `MIEL_DB_PATH` or
/// [`DatabaseStorage::new_file`] for an explicit database path.
pub struct DatabaseStorage {
    rt: tokio::runtime::Runtime,
    conn: DatabaseConnection,
}

impl DatabaseStorage {
    /// Default database filename used in the application's working directory
    const DEFAULT_DB_FILE: &'static str = "miel.sqlite3";

    /// Create or open the database using an env override if provided, otherwise in the current working directory
    pub fn new() -> Result<Self, StorageError> {
        if let Ok(path_str) = env::var("MIEL_DB_PATH") {
            let path = Path::new(&path_str);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|_| StorageError::WriteFailed)?;
            }
            info!(
                "Opening SQLite database from MIEL_DB_PATH: {}",
                path.display()
            );
            return Self::new_file(path);
        }
        let cwd = env::current_dir().map_err(|_| StorageError::ConnectionFailed)?;
        let path = cwd.join(Self::DEFAULT_DB_FILE);
        info!("Opening default SQLite database at: {}", path.display());
        Self::new_file(path)
    }

    /// Create or open the database at the specified filesystem path.
    ///
    /// The file is created if missing; parent directories are ensured.
    pub fn new_file<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| StorageError::ConnectionFailed)?;
        let path_ref = path.as_ref();
        if let Some(parent) = path_ref.parent() {
            std::fs::create_dir_all(parent).map_err(|_| StorageError::WriteFailed)?;
        }
        info!("Connecting to SQLite at: {}", path_ref.display());
        // DSN understood by sea-orm/sqlx driver; will create file if needed
        let dsn = format!("sqlite://{}?mode=rwc", path_ref.to_string_lossy());
        let conn = rt.block_on(async {
            let conn = Database::connect(dsn).await.map_err(|e| {
                error!("DB connect failed: {}", e);
                StorageError::ConnectionFailed
            })?;
            // ensure foreign keys
            conn.execute(Statement::from_string(
                DbBackend::Sqlite,
                "PRAGMA foreign_keys = ON".to_string(),
            ))
            .await
            .map_err(|e| {
                error!("Failed to enable foreign_keys PRAGMA: {}", e);
                StorageError::WriteFailed
            })?;
            // create schema
            debug!("Ensuring sessions table exists");
            conn.execute(Statement::from_string(
                DbBackend::Sqlite,
                r#"
                CREATE TABLE IF NOT EXISTS sessions (
                    id TEXT PRIMARY KEY,
                    service_name TEXT NOT NULL,
                    client_addr TEXT NOT NULL,
                    start_time TEXT NOT NULL,
                    end_time TEXT,
                    container_id TEXT,
                    bytes_transferred INTEGER NOT NULL,
                    status TEXT NOT NULL
                );
            "#
                .to_string(),
            ))
            .await
            .map_err(|e| {
                error!("Failed to create sessions table: {}", e);
                StorageError::WriteFailed
            })?;
            debug!("Ensuring interactions table exists");
            conn.execute(Statement::from_string(
                DbBackend::Sqlite,
                r#"
                CREATE TABLE IF NOT EXISTS interactions (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    session_id TEXT NOT NULL,
                    data BLOB NOT NULL,
                    FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
                );
            "#
                .to_string(),
            ))
            .await
            .map_err(|e| {
                error!("Failed to create interactions table: {}", e);
                StorageError::WriteFailed
            })?;
            debug!("Ensuring artifacts table exists");
            conn.execute(Statement::from_string(
                DbBackend::Sqlite,
                r#"
                CREATE TABLE IF NOT EXISTS artifacts (
                    session_id TEXT PRIMARY KEY,
                    json TEXT NOT NULL,
                    FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
                );
            "#
                .to_string(),
            ))
            .await
            .map_err(|e| {
                error!("Failed to create artifacts table: {}", e);
                StorageError::WriteFailed
            })?;
            Ok::<_, StorageError>(conn)
        })?;
        info!("SQLite connection ready");
        Ok(Self { rt, conn })
    }

    fn to_session_model(s: &Session) -> session::ActiveModel {
        session::ActiveModel {
            id: Set(s.id.to_string()),
            service_name: Set(s.service_name.clone()),
            client_addr: Set(s.client_addr.to_string()),
            start_time: Set(s.start_time.to_rfc3339()),
            end_time: Set(s.end_time.map(|t| t.to_rfc3339())),
            container_id: Set(s.container_id.clone()),
            bytes_transferred: Set(s.bytes_transferred as i64),
            status: Set(match s.status {
                crate::session_management::SessionStatus::Pending => "Pending",
                crate::session_management::SessionStatus::Active => "Active",
                crate::session_management::SessionStatus::Completed => "Completed",
                crate::session_management::SessionStatus::Error => "Error",
            }
            .to_string()),
        }
    }

    fn from_session_model(m: session::Model) -> Result<Session, StorageError> {
        let status = match m.status.as_str() {
            "Pending" => crate::session_management::SessionStatus::Pending,
            "Active" => crate::session_management::SessionStatus::Active,
            "Completed" => crate::session_management::SessionStatus::Completed,
            _ => crate::session_management::SessionStatus::Error,
        };
        Ok(Session {
            id: Uuid::parse_str(&m.id).map_err(|_| StorageError::ReadFailed)?,
            service_name: m.service_name,
            client_addr: m
                .client_addr
                .parse()
                .map_err(|_| StorageError::ReadFailed)?,
            start_time: DateTime::parse_from_rfc3339(&m.start_time)
                .map_err(|_| StorageError::ReadFailed)?
                .with_timezone(&Utc),
            end_time: match m.end_time {
                Some(s) => Some(
                    DateTime::parse_from_rfc3339(&s)
                        .map_err(|_| StorageError::ReadFailed)?
                        .with_timezone(&Utc),
                ),
                None => None,
            },
            container_id: m.container_id,
            bytes_transferred: m.bytes_transferred as u64,
            status,
        })
    }
}

impl Storage for DatabaseStorage {
    fn save_session(&self, session_obj: &Session) -> Result<(), StorageError> {
        let mut am = Self::to_session_model(session_obj);
        self.rt.block_on(async {
            match session::Entity::find_by_id(session_obj.id.to_string())
                .one(&self.conn)
                .await
                .map_err(|e| {
                    error!("DB read error in save_session find_by_id: {}", e);
                    StorageError::ReadFailed
                })? {
                Some(existing) => {
                    am.id = Set(existing.id);
                    am.update(&self.conn).await.map_err(|e| {
                        error!("DB write error in save_session update: {}", e);
                        StorageError::WriteFailed
                    })?;
                    info!("Updated session {}", session_obj.id);
                }
                None => {
                    // Use exec to avoid fetching inserted row (SQLite RETURNING may be unavailable)
                    session::Entity::insert(am)
                        .exec(&self.conn)
                        .await
                        .map_err(|e| {
                            error!("DB write error in save_session insert exec: {}", e);
                            StorageError::WriteFailed
                        })?;
                    info!("Inserted session {}", session_obj.id);
                }
            }
            Ok(())
        })
    }

    fn get_sessions(&self, filter: Option<SessionFilter>) -> Result<Vec<Session>, StorageError> {
        self.rt.block_on(async {
            let mut query = session::Entity::find();
            if filter.is_some() {
                debug!("Applying session filter");
            }
            if let Some(f) = filter {
                let mut cond = Condition::all();
                if let Some(name) = f.service_name {
                    cond = cond.add(session::Column::ServiceName.eq(name));
                }
                if let Some(start) = f.start_date {
                    cond = cond.add(session::Column::StartTime.gte(start.to_rfc3339()));
                }
                if let Some(end) = f.end_date {
                    let coalesce = Func::coalesce([
                        Expr::col(session::Column::EndTime).into(),
                        Expr::col(session::Column::StartTime).into(),
                    ]);
                    cond = cond.add(Expr::expr(coalesce).lte(end.to_rfc3339()));
                }
                if let Some(ip) = f.client_addr {
                    cond = cond.add(session::Column::ClientAddr.like(format!("{}:%", ip)));
                }
                if let Some(st) = f.status {
                    let s = match st {
                        crate::session_management::SessionStatus::Pending => "Pending",
                        crate::session_management::SessionStatus::Active => "Active",
                        crate::session_management::SessionStatus::Completed => "Completed",
                        crate::session_management::SessionStatus::Error => "Error",
                    };
                    cond = cond.add(session::Column::Status.eq(s));
                }
                query = query.filter(cond);
            }
            let rows = query.all(&self.conn).await.map_err(|e| {
                error!("DB read error in get_sessions: {}", e);
                StorageError::ReadFailed
            })?;
            debug!("Fetched {} session rows", rows.len());
            rows.into_iter().map(Self::from_session_model).collect()
        })
    }

    fn save_interaction(&self, session_id: Uuid, data: &[u8]) -> Result<(), StorageError> {
        self.rt.block_on(async {
            let am = inter::ActiveModel {
                session_id: Set(session_id.to_string()),
                data: Set(data.to_vec()),
                ..Default::default()
            };
            am.insert(&self.conn).await.map_err(|e| {
                error!("DB write error in save_interaction insert: {}", e);
                StorageError::WriteFailed
            })?;
            debug!(
                "Inserted interaction ({} bytes) for session {}",
                data.len(),
                session_id
            );
            Ok(())
        })
    }

    fn get_session_data(&self, session_id: Uuid) -> Result<Vec<u8>, StorageError> {
        self.rt.block_on(async {
            let mut out = Vec::new();
            let rows = inter::Entity::find()
                .filter(inter::Column::SessionId.eq(session_id.to_string()))
                .order_by_asc(inter::Column::Id)
                .all(&self.conn)
                .await
                .map_err(|e| {
                    error!("DB read error in get_session_data: {}", e);
                    StorageError::ReadFailed
                })?;
            let mut chunks = 0usize;
            for r in rows {
                out.extend_from_slice(&r.data);
                chunks += 1;
            }
            debug!(
                "Concatenated {} interaction chunk(s) for session {}, total {} bytes",
                chunks,
                session_id,
                out.len()
            );
            Ok(out)
        })
    }

    fn cleanup_old_sessions(&self, older_than: DateTime<Utc>) -> Result<usize, StorageError> {
        self.rt.block_on(async {
            let cutoff = older_than.to_rfc3339();
            let coalesce = Func::coalesce([
                Expr::col(session::Column::EndTime).into(),
                Expr::col(session::Column::StartTime).into(),
            ]);
            let cond = Expr::expr(coalesce).lt(cutoff.clone());
            let res = session::Entity::delete_many()
                .filter(cond)
                .exec(&self.conn)
                .await
                .map_err(|e| {
                    error!("DB write error in cleanup_old_sessions delete_many: {}", e);
                    StorageError::WriteFailed
                })?;
            info!(
                "Deleted {} session(s) older than {}",
                res.rows_affected, cutoff
            );
            Ok(res.rows_affected as usize)
        })
    }

    fn save_capture_artifacts(&self, artifacts: &CaptureArtifacts) -> Result<(), StorageError> {
        self.rt.block_on(async {
            let id = artifacts.session_id.to_string();
            let json = serde_json::to_string(artifacts).map_err(|_| StorageError::WriteFailed)?;
            match art::Entity::find_by_id(id.clone())
                .one(&self.conn)
                .await
                .map_err(|e| {
                    error!("DB read error in save_capture_artifacts find_by_id: {}", e);
                    StorageError::ReadFailed
                })? {
                Some(_) => {
                    let am = art::ActiveModel {
                        session_id: Set(id.clone()),
                        json: Set(json),
                    };
                    am.update(&self.conn).await.map_err(|e| {
                        error!("DB write error in save_capture_artifacts update: {}", e);
                        StorageError::WriteFailed
                    })?;
                    info!("Updated artifacts for session {}", id);
                }
                None => {
                    let am = art::ActiveModel {
                        session_id: Set(id.clone()),
                        json: Set(json),
                    };
                    art::Entity::insert(am)
                        .exec(&self.conn)
                        .await
                        .map_err(|e| {
                            error!(
                                "DB write error in save_capture_artifacts insert exec: {}",
                                e
                            );
                            StorageError::WriteFailed
                        })?;
                    info!("Inserted artifacts for session {}", id);
                }
            }
            Ok(())
        })
    }

    fn get_capture_artifacts(&self, session_id: Uuid) -> Result<CaptureArtifacts, StorageError> {
        self.rt.block_on(async {
            let id = session_id.to_string();
            let m = art::Entity::find_by_id(id.clone())
                .one(&self.conn)
                .await
                .map_err(|e| {
                    error!("DB read error in get_capture_artifacts find_by_id: {}", e);
                    StorageError::ReadFailed
                })?
                .ok_or(StorageError::ReadFailed)?;
            let artifacts: CaptureArtifacts =
                serde_json::from_str(&m.json).map_err(|_| StorageError::ReadFailed)?;
            debug!("Loaded artifacts for session {}", id);
            Ok(artifacts)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_management::SessionStatus;
    use tempfile::TempDir;

    fn temp_db() -> DatabaseStorage {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.sqlite3");
        // Keep TempDir alive by leaking it for the test duration
        Box::leak(Box::new(dir));
        DatabaseStorage::new_file(path).unwrap()
    }

    #[test]
    fn test_db_session_and_filter() {
        let storage = temp_db();
        let now = Utc::now();
        let s1 = Session {
            id: Uuid::new_v4(),
            service_name: "ssh".into(),
            client_addr: "127.0.0.1:2222".parse().unwrap(),
            start_time: now,
            end_time: Some(now),
            container_id: None,
            bytes_transferred: 100,
            status: SessionStatus::Completed,
        };
        storage.save_session(&s1).unwrap();
        let all = storage.get_sessions(None).unwrap();
        assert_eq!(all.len(), 1);
        let filtered = storage
            .get_sessions(Some(SessionFilter {
                service_name: Some("ssh".into()),
                ..Default::default()
            }))
            .unwrap();
        assert_eq!(filtered.len(), 1);
        let none = storage
            .get_sessions(Some(SessionFilter {
                service_name: Some("http".into()),
                ..Default::default()
            }))
            .unwrap();
        assert_eq!(none.len(), 0);
    }

    #[test]
    fn test_db_interactions_roundtrip() {
        let storage = temp_db();
        let id = Uuid::new_v4();
        storage
            .save_session(&Session {
                id,
                service_name: "svc".into(),
                client_addr: "127.0.0.1:1".parse().unwrap(),
                start_time: Utc::now(),
                end_time: None,
                container_id: None,
                bytes_transferred: 0,
                status: SessionStatus::Pending,
            })
            .unwrap();
        storage.save_interaction(id, b"abc").unwrap();
        storage.save_interaction(id, b"def").unwrap();
        let data = storage.get_session_data(id).unwrap();
        assert_eq!(data, b"abcdef");
    }

    #[test]
    fn test_db_artifacts_roundtrip_and_cleanup() {
        let storage = temp_db();
        let id = Uuid::new_v4();
        let now = Utc::now();
        let session = Session {
            id,
            service_name: "svc".into(),
            client_addr: "127.0.0.1:1".parse().unwrap(),
            start_time: now,
            end_time: Some(now),
            container_id: None,
            bytes_transferred: 0,
            status: SessionStatus::Completed,
        };
        storage.save_session(&session).unwrap();
        let artifacts = CaptureArtifacts {
            session_id: id,
            tcp_client_to_container: vec![1, 2],
            tcp_container_to_client: vec![3, 4],
            stdio_stdin: vec![5],
            stdio_stdout: vec![6],
            stdio_stderr: vec![7],
            tcp_timestamps: vec![],
            stdio_timestamps: vec![],
            total_bytes: 5,
            duration: chrono::Duration::seconds(1),
        };
        storage.save_capture_artifacts(&artifacts).unwrap();
        let fetched = storage.get_capture_artifacts(id).unwrap();
        assert_eq!(fetched.total_bytes, 5);
        let removed = storage
            .cleanup_old_sessions(Utc::now() + chrono::Duration::seconds(1))
            .unwrap();
        assert_eq!(removed, 1);
        let missing = storage.get_capture_artifacts(id);
        assert!(missing.is_err());
    }
}

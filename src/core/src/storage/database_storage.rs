use std::env;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Pool, Row, Sqlite,
};
use uuid::Uuid;

use crate::error_handling::types::StorageError;
use crate::storage::storage::Storage;
use crate::storage::types::{CaptureArtifacts, Session, SessionFilter};

// Internal row mapping for sessions to avoid manual try_get
#[derive(Debug, sqlx::FromRow)]
struct SessionRow {
    id: String,
    service_name: String,
    client_addr: String,
    start_time: String,
    end_time: Option<String>,
    container_id: Option<String>,
    bytes_transferred: i64,
    status: String,
}

impl SessionRow {
    fn into_session(self) -> Result<Session, StorageError> {
        let status = match self.status.as_str() {
            "Pending" => crate::session_management::SessionStatus::Pending,
            "Active" => crate::session_management::SessionStatus::Active,
            "Completed" => crate::session_management::SessionStatus::Completed,
            _ => crate::session_management::SessionStatus::Error,
        };
        Ok(Session {
            id: Uuid::parse_str(&self.id).map_err(|_| StorageError::ReadFailed)?,
            service_name: self.service_name,
            client_addr: self.client_addr.parse().map_err(|_| StorageError::ReadFailed)?,
            start_time: DateTime::parse_from_rfc3339(&self.start_time)
                .map_err(|_| StorageError::ReadFailed)?
                .with_timezone(&Utc),
            end_time: match self.end_time {
                Some(s) => Some(
                    DateTime::parse_from_rfc3339(&s)
                        .map_err(|_| StorageError::ReadFailed)?
                        .with_timezone(&Utc),
                ),
                None => None,
            },
            container_id: self.container_id,
            bytes_transferred: self.bytes_transferred as u64,
            status,
        })
    }
}

pub struct DatabaseStorage {
    rt: tokio::runtime::Runtime,
    pool: Pool<Sqlite>,
}

impl DatabaseStorage {
    /// Default database filename used in the application's working directory
    const DEFAULT_DB_FILE: &'static str = "miel.sqlite3";

    /// Create or open the database in the current working directory with the default filename
    pub fn new() -> Result<Self, StorageError> {
        let cwd = env::current_dir().map_err(|_| StorageError::ConnectionFailed)?;
        let path = cwd.join(Self::DEFAULT_DB_FILE);
        Self::new_file(path)
    }

    pub fn new_file<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| StorageError::ConnectionFailed)?;
        let path_ref = path.as_ref();
        if let Some(parent) = path_ref.parent() {
            std::fs::create_dir_all(parent).map_err(|_| StorageError::WriteFailed)?;
        }
        let pool = rt.block_on(async {
            let opts = SqliteConnectOptions::from_str("sqlite://")
                .unwrap()
                .filename(path_ref)
                .create_if_missing(true);
            let pool = SqlitePoolOptions::new()
                .max_connections(5)
                .connect_with(opts)
                .await
                .map_err(|_| StorageError::ConnectionFailed)?;
            // ensure foreign keys
            sqlx::query("PRAGMA foreign_keys = ON;")
                .execute(&pool)
                .await
                .map_err(|_| StorageError::WriteFailed)?;
            // create schema
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS sessions (
                    id TEXT PRIMARY KEY,
                    service_name TEXT NOT NULL,
                    client_addr TEXT NOT NULL,
                    start_time TEXT NOT NULL,
                    end_time TEXT,
                    container_id TEXT,
                    bytes_transferred INTEGER NOT NULL,
                    status TEXT NOT NULL
                );",
            )
            .execute(&pool)
            .await
            .map_err(|_| StorageError::WriteFailed)?;
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS interactions (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    session_id TEXT NOT NULL,
                    data BLOB NOT NULL,
                    FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
                );",
            )
            .execute(&pool)
            .await
            .map_err(|_| StorageError::WriteFailed)?;
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS artifacts (
                    session_id TEXT PRIMARY KEY,
                    json TEXT NOT NULL,
                    FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
                );",
            )
            .execute(&pool)
            .await
            .map_err(|_| StorageError::WriteFailed)?;
            Ok::<_, StorageError>(pool)
        })?;
        Ok(Self { rt, pool })
    }
}

impl Storage for DatabaseStorage {
    fn save_session(&self, session: &Session) -> Result<(), StorageError> {
        self.rt.block_on(async {
            sqlx::query(
                "INSERT INTO sessions (id, service_name, client_addr, start_time, end_time, container_id, bytes_transferred, status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET
                   service_name=excluded.service_name,
                   client_addr=excluded.client_addr,
                   start_time=excluded.start_time,
                   end_time=excluded.end_time,
                   container_id=excluded.container_id,
                   bytes_transferred=excluded.bytes_transferred,
                   status=excluded.status",
            )
            .bind(session.id.to_string())
            .bind(&session.service_name)
            .bind(session.client_addr.to_string())
            .bind(session.start_time.to_rfc3339())
            .bind(session.end_time.map(|d| d.to_rfc3339()))
            .bind(session.container_id.clone())
            .bind(session.bytes_transferred as i64)
            .bind(match session.status {
                crate::session_management::SessionStatus::Pending => "Pending",
                crate::session_management::SessionStatus::Active => "Active",
                crate::session_management::SessionStatus::Completed => "Completed",
                crate::session_management::SessionStatus::Error => "Error",
            })
            .execute(&self.pool)
            .await
            .map_err(|_| StorageError::WriteFailed)?;
            Ok(())
        })
    }

    fn get_sessions(&self, filter: Option<SessionFilter>) -> Result<Vec<Session>, StorageError> {
        self.rt.block_on(async {
            let mut sql = String::from(
                "SELECT id, service_name, client_addr, start_time, end_time, container_id, bytes_transferred, status FROM sessions",
            );
            let mut clauses: Vec<String> = Vec::new();
            let mut binds: Vec<String> = Vec::new();
            if let Some(f) = &filter {
                if let Some(ref name) = f.service_name {
                    clauses.push("service_name = ?".into());
                    binds.push(name.clone());
                }
                if let Some(start) = f.start_date {
                    clauses.push("start_time >= ?".into());
                    binds.push(start.to_rfc3339());
                }
                if let Some(end) = f.end_date {
                    clauses.push("COALESCE(end_time, start_time) <= ?".into());
                    binds.push(end.to_rfc3339());
                }
                if let Some(ip) = f.client_addr {
                    clauses.push("client_addr LIKE ?".into());
                    binds.push(format!("{}:%", ip));
                }
                if let Some(ref st) = f.status {
                    let s = match st {
                        crate::session_management::SessionStatus::Pending => "Pending",
                        crate::session_management::SessionStatus::Active => "Active",
                        crate::session_management::SessionStatus::Completed => "Completed",
                        crate::session_management::SessionStatus::Error => "Error",
                    };
                    clauses.push("status = ?".into());
                    binds.push(s.into());
                }
            }
            if !clauses.is_empty() {
                sql.push_str(" WHERE ");
                sql.push_str(&clauses.join(" AND "));
            }

            let mut q = sqlx::query_as::<_, SessionRow>(&sql);
            for b in &binds {
                q = q.bind(b);
            }
            let rows: Vec<SessionRow> =
                q.fetch_all(&self.pool).await.map_err(|_| StorageError::ReadFailed)?;
            let mut out = Vec::with_capacity(rows.len());
            for row in rows {
                out.push(row.into_session()?);
            }
            Ok(out)
        })
    }

    fn save_interaction(&self, session_id: Uuid, data: &[u8]) -> Result<(), StorageError> {
        self.rt.block_on(async {
            sqlx::query("INSERT INTO interactions (session_id, data) VALUES (?1, ?2)")
                .bind(session_id.to_string())
                .bind(data)
                .execute(&self.pool)
                .await
                .map_err(|_| StorageError::WriteFailed)?;
            Ok(())
        })
    }

    fn get_session_data(&self, session_id: Uuid) -> Result<Vec<u8>, StorageError> {
        self.rt.block_on(async {
            let rows = sqlx::query(
                "SELECT data FROM interactions WHERE session_id = ?1 ORDER BY id ASC",
            )
            .bind(session_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(|_| StorageError::ReadFailed)?;
            let mut out = Vec::new();
            for row in rows {
                let chunk: Vec<u8> =
                    row.try_get::<Vec<u8>, _>(0).map_err(|_| StorageError::ReadFailed)?;
                out.extend_from_slice(&chunk);
            }
            Ok(out)
        })
    }

    fn cleanup_old_sessions(&self, older_than: DateTime<Utc>) -> Result<usize, StorageError> {
        self.rt.block_on(async {
            let cutoff = older_than.to_rfc3339();
            // Count first
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM sessions WHERE COALESCE(end_time, start_time) < ?1",
            )
            .bind(&cutoff)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| StorageError::ReadFailed)?;
            // Delete
            sqlx::query(
                "DELETE FROM sessions WHERE COALESCE(end_time, start_time) < ?1",
            )
            .bind(&cutoff)
            .execute(&self.pool)
            .await
            .map_err(|_| StorageError::WriteFailed)?;
            Ok(count as usize)
        })
    }

    fn save_capture_artifacts(&self, artifacts: &CaptureArtifacts) -> Result<(), StorageError> {
        self.rt.block_on(async {
            let json = serde_json::to_string(artifacts).map_err(|_| StorageError::WriteFailed)?;
            sqlx::query(
                "INSERT INTO artifacts (session_id, json) VALUES (?1, ?2) ON CONFLICT(session_id) DO UPDATE SET json = excluded.json",
            )
            .bind(artifacts.session_id.to_string())
            .bind(json)
            .execute(&self.pool)
            .await
            .map_err(|_| StorageError::WriteFailed)?;
            Ok(())
        })
    }

    fn get_capture_artifacts(&self, session_id: Uuid) -> Result<CaptureArtifacts, StorageError> {
        self.rt.block_on(async {
            let json: String = sqlx::query_scalar(
                "SELECT json FROM artifacts WHERE session_id = ?1",
            )
            .bind(session_id.to_string())
            .fetch_one(&self.pool)
            .await
            .map_err(|_| StorageError::ReadFailed)?;
            let artifacts: CaptureArtifacts =
                serde_json::from_str(&json).map_err(|_| StorageError::ReadFailed)?;
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
        let path: PathBuf = dir.path().join("test.sqlite3");
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

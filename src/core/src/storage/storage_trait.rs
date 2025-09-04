//! Storage Trait
//!
//! This module defines the `Storage` trait, which provides an interface for session and artifact storage backends.
//!
//! Implementors of this trait are responsible for:
//! - Persisting and retrieving session data
//! - Managing interaction data
//! - Handling capture artifacts
//! - Cleaning up old sessions
//!
//! All methods return a `Result` to handle potential storage errors.

use crate::error_handling::types::StorageError;
use crate::session::Session;
use crate::storage::types::{CaptureArtifacts, SessionFilter};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// The `Storage` trait defines the interface for session and artifact storage backends.
///
/// Implementors of this trait are responsible for persisting and retrieving session data,
/// interaction data, and capture artifacts, as well as cleaning up old sessions.
///
/// All methods return a `Result` to handle potential storage errors.
pub trait Storage: Send + Sync {
    /// Saves a session to the storage backend.
    ///
    /// - `session` - The `Session` to be saved.
    fn save_session(&self, session: &Session) -> Result<(), StorageError>;

    /// Retrieves sessions, optionally filtered.
    fn get_sessions(&self, filter: Option<SessionFilter>) -> Result<Vec<Session>, StorageError>;

    /// Saves interaction data for a given session.
    fn save_interaction(&self, session_id: Uuid, data: &[u8]) -> Result<(), StorageError>;

    /// Retrieves all interaction data for a given session.
    fn get_session_data(&self, session_id: Uuid) -> Result<Vec<u8>, StorageError>;

    /// Cleans up sessions older than the specified date and time.
    fn cleanup_old_sessions(&self, older_than: DateTime<Utc>) -> Result<usize, StorageError>;

    /// Saves capture artifacts to the storage backend.
    fn save_capture_artifacts(&self, artifacts: &CaptureArtifacts) -> Result<(), StorageError>;

    /// Retrieves capture artifacts for a given session.
    fn get_capture_artifacts(&self, session_id: Uuid) -> Result<CaptureArtifacts, StorageError>;
}

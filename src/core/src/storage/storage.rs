//! # Storage Trait
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
use crate::storage::types::{CaptureArtifacts, Session, SessionFilter};
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
    /// # Arguments
    ///
    /// * `session` - A reference to the `Session` to be saved.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError` if the operation fails.
    fn save_session(&self, session: &Session) -> Result<(), StorageError>;

    /// Retrieves sessions from the storage backend, optionally filtered.
    ///
    /// # Arguments
    ///
    /// * `filter` - An optional `SessionFilter` to filter sessions.
    ///
    /// # Returns
    ///
    /// A vector of `Session` objects.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError` if the operation fails.
    fn get_sessions(&self, filter: Option<SessionFilter>) -> Result<Vec<Session>, StorageError>;

    /// Saves interaction data for a given session.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The UUID of the session.
    /// * `data` - The interaction data as a byte slice.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError` if the operation fails.
    fn save_interaction(&self, session_id: Uuid, data: &[u8]) -> Result<(), StorageError>;

    /// Retrieves all interaction data for a given session.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The UUID of the session.
    ///
    /// # Returns
    ///
    /// The session data as a vector of bytes.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError` if the operation fails.
    fn get_session_data(&self, session_id: Uuid) -> Result<Vec<u8>, StorageError>;

    /// Cleans up sessions older than the specified date and time.
    ///
    /// # Arguments
    ///
    /// * `older_than` - A `DateTime<Utc>` specifying the cutoff.
    ///
    /// # Returns
    ///
    /// The number of sessions deleted.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError` if the operation fails.
    fn cleanup_old_sessions(&self, older_than: DateTime<Utc>) -> Result<usize, StorageError>;

    /// Saves capture artifacts to the storage backend.
    ///
    /// # Arguments
    ///
    /// * `artifacts` - A reference to the `CaptureArtifacts` to be saved.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError` if the operation fails.
    fn save_capture_artifacts(&self, artifacts: &CaptureArtifacts) -> Result<(), StorageError>;

    /// Retrieves capture artifacts for a given session.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The UUID of the session.
    ///
    /// # Returns
    ///
    /// The `CaptureArtifacts` associated with the session.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError` if the operation fails.
    fn get_capture_artifacts(&self, session_id: Uuid) -> Result<CaptureArtifacts, StorageError>;
}
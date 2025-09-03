//! Persistence abstraction for capture artifacts.
//!
//! Implement this trait to store and retrieve [`CaptureArtifacts`]
//! to your preferred backend (filesystem, database, object storage, etc.).

use super::types::CaptureArtifacts;
use crate::error_handling::types::StorageError;

/// Storage backend contract for persisted capture artifacts.
pub trait Storage: Send + Sync {
    /// Persist the provided capture artifacts.
    ///
    /// Errors
    /// - Returns [`StorageError`] if the backend cannot save the artifacts.
    fn save_capture_artifacts(&self, artifacts: &CaptureArtifacts) -> Result<(), StorageError>;

    /// Fetch previously saved artifacts for a session.
    ///
    /// Errors
    /// - Returns [`StorageError`] if the session is missing or retrieval fails.
    fn get_capture_artifacts(
        &self,
        session_id: uuid::Uuid,
    ) -> Result<CaptureArtifacts, StorageError>;
}

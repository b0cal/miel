// Storage trait as per UML
use crate::error_handling::types::StorageError;
use crate::storage::types::{CaptureArtifacts, Session, SessionFilter};
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub trait Storage: Send + Sync {
    fn save_session(&self, session: &Session) -> Result<(), StorageError>;
    fn get_sessions(&self, filter: Option<SessionFilter>) -> Result<Vec<Session>, StorageError>;
    fn save_interaction(&self, session_id: Uuid, data: &[u8]) -> Result<(), StorageError>;
    fn get_session_data(&self, session_id: Uuid) -> Result<Vec<u8>, StorageError>;
    fn cleanup_old_sessions(&self, older_than: DateTime<Utc>) -> Result<usize, StorageError>;
    fn save_capture_artifacts(&self, artifacts: &CaptureArtifacts) -> Result<(), StorageError>;
    fn get_capture_artifacts(&self, session_id: Uuid) -> Result<CaptureArtifacts, StorageError>;
}


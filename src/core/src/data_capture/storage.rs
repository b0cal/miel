use super::types::CaptureArtifacts;
use crate::error_handling::types::StorageError;

pub trait Storage: Send + Sync {
    fn save_capture_artifacts(&self, artifacts: &CaptureArtifacts) -> Result<(), StorageError>;
    fn get_capture_artifacts(
        &self,
        session_id: uuid::Uuid,
    ) -> Result<CaptureArtifacts, StorageError>;
}

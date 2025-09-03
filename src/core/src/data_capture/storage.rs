use crate::error_handling::types::StorageError;
use super::types::CaptureArtifacts;

pub trait Storage: Send + Sync {
    fn save_capture_artifacts(
        &self,
        artifacts: &CaptureArtifacts,
    ) -> Result<(), StorageError>;
    fn get_capture_artifacts(
        &self,
        session_id: uuid::Uuid,
    ) -> Result<CaptureArtifacts, StorageError>;
}


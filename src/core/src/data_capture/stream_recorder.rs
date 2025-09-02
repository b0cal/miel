use crate::storage::Storage;

pub struct StreamRecorder {
    session_id: uuid::Uuid,
    storage: std::sync::Arc<dyn Storage>,
    // Add other fields as necessary
}

impl StreamRecorder {
    pub fn new(session_id: uuid::Uuid, storage: std::sync::Arc<dyn Storage>) -> Self {
        StreamRecorder {
            session_id,
            storage,
            // Initialize other fields as needed, or use default values
        }
    }
}

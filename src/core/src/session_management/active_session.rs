use crate::container_management::ContainerHandle;
use crate::data_capture::StreamRecorder;
use crate::session_management::session::Session;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Represents an active session, containing the session state,
/// an optional handle to a running container, and a stream recorder
/// for capturing data during the session.
pub struct ActiveSession {
    /// The session metadata and state.
    pub session: Session,
    /// Optional handle to the associated container, if any.
    pub container_handle: Option<ContainerHandle>,
    /// Recorder for capturing streaming data during the session.
    /// Wrapped in Arc<Mutex<>> for thread-safe access across async contexts.
    pub stream_recorder: Arc<Mutex<StreamRecorder>>,
}

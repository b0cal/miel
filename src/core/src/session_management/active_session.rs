use crate::container_management::ContainerHandle;
use crate::data_capture::StreamRecorder;
use crate::session_management::session::Session;

pub struct ActiveSession {
    // Fields for the ActiveSession struct
    pub session: Session,
    pub container_handle: Option<ContainerHandle>,
    pub stream_recorder: StreamRecorder,
}

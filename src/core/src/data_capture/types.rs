use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    ClientToContainer,
    ContainerToClient,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdioStream {
    Stdin,
    Stdout,
    Stderr,
}

#[derive(Debug, Clone)]
pub struct CaptureArtifacts {
    pub session_id: Uuid,
    pub tcp_client_to_container: Vec<u8>,
    pub tcp_container_to_client: Vec<u8>,
    pub stdio_stdin: Vec<u8>,
    pub stdio_stdout: Vec<u8>,
    pub stdio_stderr: Vec<u8>,
    pub tcp_timestamps: Vec<(DateTime<Utc>, Direction, usize)>,
    pub stdio_timestamps: Vec<(DateTime<Utc>, StdioStream, usize)>,
    pub total_bytes: u64,
    pub duration: chrono::Duration,
}


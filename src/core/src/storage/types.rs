use chrono::{DateTime, Duration, Utc};
use std::net::{IpAddr, SocketAddr};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

// FIXME: use actual types once they are correctly exported from their modules

// Reuse the SessionStatus enum from session_management
use crate::session_management::SessionStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub service_name: String,
    pub client_addr: SocketAddr,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub container_id: Option<String>,
    pub bytes_transferred: u64,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Direction {
    ClientToContainer,
    ContainerToClient,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StdioStream {
    Stdin,
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub duration: Duration,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionFilter {
    pub service_name: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub client_addr: Option<IpAddr>,
    pub status: Option<SessionStatus>,
}

//! Common data types used across the data_capture subsystem.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Direction of TCP flow for captured bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    /// Bytes flowing from the external client to the container/service.
    ClientToContainer,
    /// Bytes flowing from the container/service back to the external client.
    ContainerToClient,
}

/// Logical stdio stream identifiers when parsing activity logs or PTY snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StdioStream {
    /// Data written by the client (e.g., typed commands), i.e., STDIN.
    Stdin,
    /// Data produced by the service on standard output.
    Stdout,
    /// Data produced by the service on standard error.
    Stderr,
}

//// Aggregated capture artifacts persisted after a session completes.
///
/// Binary payloads are stored directly; timestamp series provide lightweight
/// metadata usable for replay or analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureArtifacts {
    /// The related session identifier
    pub session_id: Uuid,
    /// Raw TCP payload captured from client to container
    pub tcp_client_to_container: Vec<u8>,
    /// Raw TCP payload captured from container to client
    pub tcp_container_to_client: Vec<u8>,
    /// Raw stdin bytes
    pub stdio_stdin: Vec<u8>,
    /// Raw stdout bytes
    pub stdio_stdout: Vec<u8>,
    /// Raw stderr bytes
    pub stdio_stderr: Vec<u8>,
    /// Timestamped sizes for TCP chunks
    pub tcp_timestamps: Vec<(DateTime<Utc>, Direction, usize)>,
    /// Timestamped sizes for STDIO chunks
    pub stdio_timestamps: Vec<(DateTime<Utc>, StdioStream, usize)>,
    /// Total number of bytes captured across channels
    pub total_bytes: u64,
    /// Total capture duration
    pub duration: Duration,
}

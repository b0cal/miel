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

/// Aggregated capture artifacts persisted after a session completes.
/// TCP payloads are stored as raw bytes for protocol analysis,
/// while STDIO streams are stored as UTF-8 text for readability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureArtifacts {
    /// The related session identifier
    pub session_id: Uuid,
    /// Raw TCP payload captured from client to container
    pub tcp_client_to_container: Vec<u8>,
    /// Raw TCP payload captured from container to client
    pub tcp_container_to_client: Vec<u8>,
    /// Decoded stdin text
    pub stdio_stdin: String,
    /// Decoded stdout text
    pub stdio_stdout: String,
    /// Decoded stderr text
    pub stdio_stderr: String,
    /// Timestamped sizes for TCP chunks
    pub tcp_timestamps: Vec<(DateTime<Utc>, Direction, usize)>,
    /// Timestamped sizes for STDIO chunks
    pub stdio_timestamps: Vec<(DateTime<Utc>, StdioStream, usize)>,
    /// Total number of bytes captured across channels
    pub total_bytes: u64,
    /// Total capture duration
    pub duration: Duration,
}

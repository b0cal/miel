//! Common data types used across the data_capture subsystem.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Direction of TCP flow for captured bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Bytes flowing from the external client to the container/service.
    ClientToContainer,
    /// Bytes flowing from the container/service back to the external client.
    ContainerToClient,
}

/// Logical stdio stream identifiers when parsing activity logs or PTY snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdioStream {
    /// Data written by the client (e.g., typed commands), i.e., STDIN.
    Stdin,
    /// Data produced by the service on standard output.
    Stdout,
    /// Data produced by the service on standard error.
    Stderr,
}

/// Aggregated artifacts for a session, combining TCP and stdio data with metadata.
#[derive(Debug, Clone)]
pub struct CaptureArtifacts {
    /// Unique session identifier.
    pub session_id: Uuid,
    /// Raw bytes sent from client to container over TCP.
    pub tcp_client_to_container: Vec<u8>,
    /// Raw bytes sent from container to client over TCP.
    pub tcp_container_to_client: Vec<u8>,
    /// Concatenated STDIN bytes recovered from logs/PTY (if available).
    pub stdio_stdin: Vec<u8>,
    /// Concatenated STDOUT bytes recovered from logs/PTY (if available).
    pub stdio_stdout: Vec<u8>,
    /// Concatenated STDERR bytes recovered from logs/PTY (if available).
    pub stdio_stderr: Vec<u8>,
    /// Per-chunk TCP timestamps with direction and byte count.
    pub tcp_timestamps: Vec<(DateTime<Utc>, Direction, usize)>,
    /// Per-line stdio timestamps with stream identifier and byte count.
    pub stdio_timestamps: Vec<(DateTime<Utc>, StdioStream, usize)>,
    /// Total captured byte count across all channels.
    pub total_bytes: u64,
    /// Wall-clock duration of the session.
    pub duration: chrono::Duration,
}

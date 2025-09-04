//! Types shared by storage backends.
//!
//! This module defines plain data types used by the `Storage` trait and its
//! implementations. These types are serializable and suitable for both
//! database and filesystem persistence.

use crate::session_management::SessionStatus;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use uuid::Uuid;

/// Direction of TCP traffic in a capture artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Direction {
    /// From client to container/service
    ClientToContainer,
    /// From container/service to client
    ContainerToClient,
}

/// STDIO stream selector in a capture artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StdioStream {
    /// Process stdin
    Stdin,
    /// Process stdout
    Stdout,
    /// Process stderr
    Stderr,
}

/// Aggregated capture artifacts persisted after a session completes.
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

/// Criteria for filtering session queries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionFilter {
    /// Match by service name
    pub service_name: Option<String>,
    /// Sessions starting at or after this time
    pub start_date: Option<DateTime<Utc>>,
    /// Sessions ending at or before this time (end_time coalesces to start_time if absent)
    pub end_date: Option<DateTime<Utc>>,
    /// Match sessions by client IP address
    pub client_addr: Option<IpAddr>,
    /// Match by final session status
    pub status: Option<SessionStatus>,
}

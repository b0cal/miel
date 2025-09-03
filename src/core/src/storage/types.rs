//! Types shared by storage backends.
//!
//! This module defines plain data types used by the `Storage` trait and its
//! implementations. These types are serializable and suitable for both
//! database and filesystem persistence.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use uuid::Uuid;

// FIXME: use actual types once they are correctly exported from their modules

// Reuse the SessionStatus enum from session_management
use crate::session_management::SessionStatus;

/// Represents a captured service session.
///
/// A session is the top-level record tying together connection metadata,
/// lifecycle timestamps, traffic accounting, and final status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique identifier for the session
    pub id: Uuid,
    /// Service name inferred or configured (e.g. "ssh", "http")
    pub service_name: String,
    /// Client socket address (IP:port)
    pub client_addr: SocketAddr,
    /// Session start time (UTC)
    pub start_time: DateTime<Utc>,
    /// Session end time (UTC), if known
    pub end_time: Option<DateTime<Utc>>,
    /// Optional container identifier running the service
    pub container_id: Option<String>,
    /// Total number of bytes transferred during the session
    pub bytes_transferred: u64,
    /// Final status of the session
    pub status: SessionStatus,
}

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

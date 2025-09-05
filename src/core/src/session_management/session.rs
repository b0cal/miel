use crate::SessionStatus;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;

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

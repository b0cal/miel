//! Types shared by storage backends.
//!
//! This module defines plain data types used by the `Storage` trait and its
//! implementations. These types are serializable and suitable for both
//! database and filesystem persistence.

use crate::session_management::SessionStatus;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

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

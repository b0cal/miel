//! Core types used by the container management subsystem.

use chrono::{DateTime, Utc};
use std::fs::File;
use tokio::net::TcpStream;

/// Aggregate counters describing the current and historical container state.
#[derive(Debug, Clone)]
pub struct ContainerStats {
    /// Number of containers currently tracked as active.
    pub active_count: usize,
    /// Total number of containers successfully created since manager init.
    pub total_created: u64,
    /// Number of operations that failed (e.g., cleanup or start failures).
    pub failed_count: u64,
}

/// Handle describing a specific container instance managed by the system.
#[derive(Debug)]
pub struct ContainerHandle {
    /// Unique identifier for the container (e.g., `miel-<service>-<uuid>`).
    pub id: String,
    /// Logical service name that the container is running (e.g., "ssh").
    pub service_name: String,
    /// Fixed internal container port where the service listens.
    pub port: u16,
    /// Ephemeral host port mapped to the container's internal `port`.
    pub host_port: u16, // ephemeral host port mapped to the container
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Process handle for the runtime container process, if available.
    pub process_handle: Option<tokio::process::Child>,
    /// Optional PTY master used to capture stdio from the container.
    pub pty_master: Option<File>,
    /// Optional TCP socket associated to the service connection lifecycle.
    pub tcp_socket: Option<TcpStream>,
}

// Implement Clone manually since tokio::process::Child and File don't implement Clone
impl Clone for ContainerHandle {
    fn clone(&self) -> Self {
        ContainerHandle {
            id: self.id.clone(),
            service_name: self.service_name.clone(),
            port: self.port,
            host_port: self.host_port,
            created_at: self.created_at,
            process_handle: None, // Can't clone process handle
            pty_master: None,     // Can't clone file handle
            tcp_socket: None,     // Can't clone TCP stream
        }
    }
}

/// Supported container runtime backends.
#[derive(Debug, Clone)]
pub enum Runtime {
    /// systemd-nspawn based containers.
    SystemdNspawn,
}

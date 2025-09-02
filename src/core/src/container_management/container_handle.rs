use super::types::ContainerStats;
use chrono::{DateTime, Utc};
use std::fs::File;
use tokio::net::TcpStream;
use tokio::process::Child;

pub struct ContainerHandle {
    pub id: String,
    pub service_name: String,
    pub port: u16,
    pub created_at: DateTime<Utc>,
    pub process_handle: Option<Child>,
    pub pty_master: Option<File>,
    pub tcp_socket: Option<TcpStream>,
}

impl Default for ContainerStats {
    fn default() -> Self {
        ContainerStats {
            active_count: 0,
            total_created: 0,
            failed_count: 0,
        }
    }
}

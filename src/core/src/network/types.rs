use crate::configuration::types::Protocol;
use chrono::{DateTime, Utc};
use std::net::SocketAddr;
use tokio::net::TcpStream;
use crate::container_management::container_handle::ContainerHandle;

#[derive(Clone)]
pub struct ServicePattern {
    pub service_name: String,
    pub port: u16,
    pub protocol: Protocol,
    pub header_patterns: Vec<String>,
    pub banner_patterns: Vec<String>,
}

pub struct SessionRequest {
    pub stream: Option<TcpStream>,
    pub service_name: String,
    pub client_addr: SocketAddr,
    pub timestamp: DateTime<Utc>,
}

impl SessionRequest {
    pub fn take_stream(&mut self) -> Option<TcpStream> {
        self.stream.take()
    }
}
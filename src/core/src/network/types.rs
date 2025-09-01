use crate::configuration::types::Protocol;
use chrono::{DateTime, Utc};
use std::net::SocketAddr;
use tokio::net::TcpStream;

#[derive(Clone)]
pub struct ServicePattern {
    pub service_name: String,
    pub port: u16,
    pub protocol: Protocol,
    pub header_patterns: Vec<String>,
    pub banner_patterns: Vec<String>,
}

#[derive(Clone)]
pub struct SessionRequest<S = TcpStream> {
    pub stream: S,
    pub service_name: String,
    pub client_addr: SocketAddr,
    pub timestamp: DateTime<Utc>,
}

use chrono::{DateTime, Utc};
use std::net::SocketAddr;
use tokio::net::TcpStream;

pub struct SessionRequest<S = TcpStream> {
    pub stream: S,
    pub service_name: String,
    pub client_addr: SocketAddr,
    pub timestamp: DateTime<Utc>,
}

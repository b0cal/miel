use crate::SessionStatus;
use std::net::SocketAddr;
use uuid::Uuid;

pub struct Session {
    // Fields for the Session struct
    pub id: Uuid,
    pub service_name: String,
    pub client_addr: SocketAddr,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub container_id: Option<String>,
    pub bytes_transferred: u64,
    pub status: SessionStatus,
}

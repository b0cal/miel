use std::net::SocketAddr;

use serde::Serialize;
use uuid::Uuid;

use crate::SessionStatus;

#[derive(Serialize)]
pub struct SessionResponse {
    pub id: Uuid,
    pub service_name: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub client_addr: String,
    pub bytes_transfered: u64,
    pub status: SessionStatus,
}

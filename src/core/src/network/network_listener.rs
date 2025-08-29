use super::connection_filter::*;
use super::service_detector::*;
use super::session_request::*;
use crate::configuration::types::ServiceConfig;
use crate::error_handling::types::NetworkError;

use std::collections::HashMap;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::Sender;

pub struct NetworkListener {
    listeners: HashMap<u16, TcpListener>,
    session_tx: Sender<SessionRequest>,
    service_detector: ServiceDetector,
    connection_filter: ConnectionFilter,
}

impl NetworkListener {
    pub fn new() -> Self {}
    pub fn bind_services(service: &[ServiceConfig]) {}
    pub fn start_listening(session_tx: Sender<SessionRequest>) -> Result<(), NetworkError> {}
    pub fn shutdown() -> Result<(), NetworkError> {}
    fn handle_connection(stream: TcpStream, service: &str) {}
}

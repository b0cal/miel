use std::collections::HashMap;
use tokio::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use crate::active_session::ActiveSession;
use crate::container_management::container_manager::ContainerManager;
use crate::network::session_request::SessionRequest;
use crate::error_handling::types::SessionError;
use crate::session::Session;

pub struct SessionManager {
    // Fields for the SessionManager struct
    active_session: HashMap<Uuid, ActiveSession>,
    container_manager: Arc<ContainerManager>,
    storage: Arc<dyn Storage>,
    max_sessions: usize,
    session_timout: Duration
}

impl SessionManager {
    pub fn new(container_manager: Arc<ContainerManager>, storage: Arc<dyn Storage>, max_sessions: usize) -> Self {

    }

    pub fn handle_session(request: SessionRequest) -> Result<(), SessionError>{
    }

    pub fn cleanup_expired_sessions(){
    }

    pub fn get_active_session_count() -> usize {
    }

    pub fn shutdown_all_sessions() -> Result<Session, SessionError>{
    }

    fn create_session(request: SessionRequest) -> Result<Session, SessionError> {
    }

    fn setup_data_proxy(session_id: Uuid, client_stream: TcpStream, container_stream: TcpStream) -> Result<(), SessionError>{
    }
}




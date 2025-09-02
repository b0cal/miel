use crate::active_session::ActiveSession;
use crate::container_management::container_manager::ContainerManager;
use crate::data_capture::stream_recorder::StreamRecorder;
use crate::error_handling::types::SessionError;
use crate::network::types::SessionRequest;
use crate::session::Session;
use crate::storage::Storage;
use crate::SessionStatus;
use chrono::Utc;
use log::{debug, error, info};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use uuid::Uuid;

/// The structure related to session management
///
/// This structure allow to manage session requests linked to an incoming connection
/// from network's modules.
///
///
/// # Fields Overview
///
/// The configuration contains the following attributes:
/// - `active_sessions`: a list of `ActiveSessions` used in certain functions to shut them down or modify them
/// - `container_manager`: Enables “coordinated” management between sessions and containers
/// - `storage`: Store information related to a session
/// - `max_sessions`: The maximum number of sessions
/// - `session_timeout`: The maximum lifetime of a session
pub struct SessionManager {
    // Fields for the SessionManager struct
    active_sessions: HashMap<Uuid, ActiveSession>,
    container_manager: Arc<ContainerManager>,
    storage: Arc<dyn Storage>,
    max_sessions: usize,
    session_timeout: Duration,
}

impl SessionManager {
    pub fn new(
        container_manager: Arc<ContainerManager>,
        storage: Arc<dyn Storage>,
        max_sessions: usize,
    ) -> Self {
        Self {
            active_sessions: HashMap::new(),
            container_manager,
            storage,
            max_sessions,
            session_timeout: Duration::new(170000, 0), //170'000sec = ~48H
        }
    }

    pub async fn handle_session(&mut self, request: SessionRequest) -> Result<(), SessionError> {
        let session = match self.create_session(request) {
            Ok(s) => s,
            Err(_) => return Err(SessionError::CreationFailed),
        };

        let active_session = match self.active_sessions.get(&session.id) {
            Some(a) => a.clone(),
            None => return Err(SessionError::CreationFailed),
        };

        let container_handle = match &active_session.container_handle {
            Some(c) => c,
            None => return Err(SessionError::CreationFailed),
        };

        self.setup_data_proxy(session.id, request.stream, container_handle.tcp_stream)
            .await?;

        Ok(())
    }

    pub fn cleanup_expired_sessions(&mut self) {
        for active_session in self.active_sessions.values_mut() {
            if active_session.session.end_time != None
                && Utc::now() - active_session.session.end_time >= self.session_timeout
            {
                active_session.session.status = SessionStatus::Completed;
                self.active_sessions.remove(&active_session.session.id);
                self.container_manager
                    .cleanup_container(&active_session.container_handle);
            }
        }
    }

    pub fn get_active_session_count(&self) -> usize {
        self.active_sessions.len()
    }

    pub fn shutdown_all_sessions(&mut self) -> Result<(), SessionError> {
        for active_session in self.active_sessions.values_mut() {
            active_session.session.status = SessionStatus::Completed;
            match self
                .container_manager
                .cleanup_container(&active_session.container_handle)
            {
                Ok(_) => Ok(()),
                Err(e) => Err(SessionError::ContainerError(e)),
            }
        }
        Ok(())
    }

    fn create_session(&mut self, request: SessionRequest) -> Result<Session, SessionError> {
        let new_container_handle = match self.container_manager.create_container(&request) {
            Ok(container_handle) => container_handle,
            Err(e) => return Err(SessionError::ContainerError(e)),
        };

        let new_session = Session {
            id: Uuid::new_v4(),
            service_name: request.service_name,
            client_addr: request.client_addr,
            start_time: request.timestamp,
            end_time: None,
            container_id: Some(new_container_handle.id.to_string()),
            bytes_transferred: 0,
            status: SessionStatus::Active,
        };

        let session_id = new_session.id;
        let new_active_session = ActiveSession {
            session: new_session,
            container_handle: Some(new_container_handle),
            stream_recorder: StreamRecorder::new(session_id, &self.storage),
        };
        let _ = match self.active_sessions.insert(session_id, new_active_session) {
            Some(_) => Ok(self.active_sessions.get(&session_id)),
            None => Err(SessionError::CreationFailed),
        };
        Ok(match self.active_sessions.get(&session_id){
            Some(&active_session) => active_session.session,
            None => return Err(SessionError::CreationFailed),
        })
    }

    async fn setup_data_proxy(
        &self,
        session_id: Uuid,
        client_stream: TcpStream,
        container_stream: TcpStream,
    ) -> Result<(), SessionError> {
        let (mut client_read, mut client_write) = client_stream.into_split();
        let (mut container_read, mut container_write) = container_stream.into_split();

        // Client to container forwarding
        let client_to_container = async move {
            let mut buffer = [0; 4096];
            loop {
                match client_read.read(&mut buffer).await {
                    Ok(0) => {
                        debug!("client disconnected");
                        break;
                    }
                    Ok(n) => {
                        debug!("client forwarding {} bytes to container", n);
                        if container_write.write_all(&buffer[..n]).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        };

        // Container to client forwarding
        let container_to_client = async move {
            let mut buffer = [0; 4096];
            loop {
                match container_read.read(&mut buffer).await {
                    Ok(0) => {
                        debug!("container disconnected");
                        break;
                    }
                    Ok(n) => {
                        debug!("container forwarding {} bytes to client", n);
                        if client_write.write_all(&buffer[..n]).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        };

        // Run both directions concurrently
        tokio::select! {
            _ = client_to_container => {},
            _ = container_to_client => {},
        }

        info!("connection closed");

        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//     use std::sync::Arc;
//     use chrono::Duration;
//     use crate::active_session::ActiveSession;
//     use crate::container_management::container_manager::ContainerManager;
//     use crate::session_manager::SessionManager;
//     use crate::session_request::SessionRequest;
//
//     #[test]
//     fn test_create_session_manager() {
//         let container_manager = Arc::new(ContainerManager::new());
//         let storage = Arc::new(DatabaseStorage);
//         let mut session_manager = SessionManager::new(container_manager.clone(), storage.clone(), 20);
//
//         assert_eq!(session_manager.active_sessions.len(), 0);
//         assert_eq!(session_manager.session_timeout, Duration::new(170000, 0));
//         assert_eq!(session_manager.max_sessions, 20);
//     }
//
//
//
//     #[test]
//     fn test_cleanup_expired_sessions() {
//         let container_manager = Arc::new(ContainerManager::new());
//         let storage = Arc::new(DatabaseStorage);
//         let mut session_manager = SessionManager::new(container_manager.clone(), storage.clone(), 20);
//
//
//         let expired_session = ActiveSession {
//             //TODO
//         };
//         session_manager.active_sessions.insert(expired_session.session.id, expired_session);
//
//         assert_eq!(session_manager.active_sessions.len(), 1);
//         session_manager.cleanup_expired_sessions();
//         assert_eq!(session_manager.active_sessions.len(), 0);
//
//     }
//
//
//     #[test]
//     fn test_create_session() {
//         let container_manager = Arc::new(ContainerManager::new());
//         let storage = Arc::new(DatabaseStorage);
//         let mut session_manager = SessionManager::new(container_manager.clone(), storage.clone(), 20);
//         let session_request = SessionRequest{
//             //TODO
//         }
//
//         session_manager.create_session(session_request);
//         assert_eq!(session_manager.active_sessions.len(), 1);
//
//     }
//
// }
//
//
//

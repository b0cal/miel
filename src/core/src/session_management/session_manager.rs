use crate::active_session::ActiveSession;
use crate::configuration::types::ServiceConfig;
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
use tokio::io::copy_bidirectional;
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

    pub async fn handle_session(
        &mut self,
        mut request: SessionRequest,
        service_config: &ServiceConfig,
    ) -> Result<(), SessionError> {
        let session = match self.create_session(&request, service_config) {
            Ok(s) => s,
            Err(_) => return Err(SessionError::CreationFailed),
        };

        let active_session = match self.active_sessions.remove(&session.id) {
            Some(a) => a,
            None => return Err(SessionError::CreationFailed),
        };

        let mut container_handle = match active_session.container_handle {
            Some(c) => c,
            None => return Err(SessionError::CreationFailed),
        };

        let tcp_stream = match container_handle.tcp_socket.as_mut() {
            Some(s) => s, // <-- `s` is now `&mut TcpStream`
            None => {
                error!("No TCP stream available for container");
                return Err(SessionError::CreationFailed);
            }
        };

        self.setup_data_proxy(session.id, &mut request.stream, tcp_stream)
            .await?;

        Ok(())
    }

    pub fn cleanup_expired_sessions(&mut self) {
        self.active_sessions.retain(|_id, active_session| {
            if let Some(end_time) = active_session.session.end_time {
                let elapsed = Utc::now() - end_time;
                if elapsed.num_seconds() >= self.session_timeout.as_secs() as i64 {
                    active_session.session.status = SessionStatus::Completed;
                    if let Some(container_handle) = active_session.container_handle.take() {
                        if let Err(e) = self.container_manager.cleanup_container(container_handle) {
                            error!("failed to clean up container: {:?}", e);
                        }
                    }
                    return false;
                }
            }
            true
        });
    }

    pub fn get_active_session_count(&self) -> usize {
        self.active_sessions.len()
    }

    pub fn shutdown_all_sessions(&mut self) -> Result<(), SessionError> {
        for active_session in self.active_sessions.values_mut() {
            active_session.session.status = SessionStatus::Completed;

            if let Some(container_handle) = active_session.container_handle.take() {
                // Pass ownership to cleanup_container
                self.container_manager
                    .cleanup_container(container_handle)
                    .map_err(SessionError::ContainerError)?;
            }
        }
        Ok(())
    }

    fn create_session(
        &mut self,
        request: &SessionRequest,
        service_config: &ServiceConfig,
    ) -> Result<Session, SessionError> {
        let new_container_handle = match self.container_manager.create_container(service_config) {
            Ok(container_handle) => container_handle,
            Err(e) => return Err(SessionError::ContainerError(e)),
        };

        let new_session = Session {
            id: Uuid::new_v4(),
            service_name: request.service_name.clone(),
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
            stream_recorder: StreamRecorder::new(session_id, self.storage.clone()),
            _cleanup_handle: tokio::spawn(async {
                //TODO: make this, think about the best way to cleanup this mess
                todo!()
            }),
        };

        self.active_sessions.insert(session_id, new_active_session);

        let active_session = self
            .active_sessions
            .get(&session_id)
            .ok_or(SessionError::CreationFailed)?;

        Ok(active_session.session.clone())
    }

    async fn setup_data_proxy(
        &self,
        session_id: Uuid,
        mut client_stream: &mut TcpStream,
        mut container_stream: &mut TcpStream,
    ) -> Result<(), SessionError> {
        match copy_bidirectional(&mut client_stream, &mut container_stream).await {
            Ok((from_client, from_container)) => {
                debug!(
                    "connection closed ({} bytes client->container, {} bytes container->client)",
                    from_client, from_container
                );
            }
            Err(e) => {
                error!("proxy error: {:?}", e);
            }
        }

        info!("proxy session ended");

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

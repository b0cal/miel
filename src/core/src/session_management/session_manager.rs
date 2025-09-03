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
use log::{debug, error};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::io::{self};
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
        request: SessionRequest,
        service_config: &ServiceConfig,
    ) -> Result<(), SessionError> {

        if self.find_session(&request).is_some() {
            debug!("Session already active for Ip:{} / Service:{}", request.client_addr.ip(), request.service_name);
            return Ok(());
        }

        let _active_session = match self.create_session(request, service_config) {
            Ok(s) => s,
            Err(_) => return Err(SessionError::CreationFailed),
        };

        Ok(())
    }

    fn find_session(&self, request: &SessionRequest) -> Option<&ActiveSession> {
        let found = self.active_sessions.iter().find(|(_, active_s)| request.client_addr.ip() == active_s.session.client_addr.ip() && request.client_addr.port() == active_s.session.client_addr.port() );
        match found {
            Some((_, active_s)) => Some(active_s),
            None => None,
        }
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

    async fn join_streams(s1: TcpStream, s2: TcpStream) -> io::Result<()> {
        let (mut r1, mut w1) = s1.into_split();
        let (mut r2, mut w2) = s2.into_split();

        let c2s = tokio::spawn(async move {
            io::copy(&mut r1, &mut w2).await
        });

        let s2c = tokio::spawn(async move {
            io::copy(&mut r2, &mut w1).await
        });

        let _ = tokio::try_join!(c2s, s2c)?;

        Ok(())
    }

    fn create_session(
        &mut self,
        mut request: SessionRequest,
        service_config: &ServiceConfig,
    ) -> Result<(), SessionError> {
        let mut container_handle = match self.container_manager.create_container(service_config) {
            Ok(container_handle) => container_handle,
            Err(e) => return Err(SessionError::ContainerError(e)),
        };

        let new_session = Session {
            id: Uuid::new_v4(),
            service_name: request.service_name.clone(),
            client_addr: request.client_addr,
            start_time: request.timestamp,
            end_time: None,
            container_id: Some(container_handle.id.to_string()),
            bytes_transferred: 0,
            status: SessionStatus::Active,
        };

        let session_id = new_session.id;

        let s1 = request.take_stream().unwrap();
        let s2 = container_handle.take_stream().unwrap();

        let join_handle = tokio::spawn(async move {
            Self::join_streams(s1, s2).await;
        });

        let new_active_session = ActiveSession {
            session: new_session,
            container_handle: Some(container_handle),
            stream_recorder: StreamRecorder::new(session_id, self.storage.clone()),
            _cleanup_handle: join_handle,
        };

        self.active_sessions.insert(session_id, new_active_session);


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

// async fn link_sockets(client: TcpStream, container: TcpStream, service: String) {
//     let (mut client_read, mut client_write) = client.into_split();
//     let (mut container_read, mut container_write) = container.into_split();
//
//     let service_client = service.clone();
//     let service_container = service.clone();
//
//     // Client to container forwarding
//     let client_to_container = async move {
//         let mut buffer = [0; 4096];
//         loop {
//             match client_read.read(&mut buffer).await {
//                 Ok(0) => {
//                     debug!("{} client disconnected", service_client);
//                     break;
//                 }
//                 Ok(n) => {
//                     debug!("{} forwarding {} bytes to container", service_client, n);
//                     if container_write.write_all(&buffer[..n]).await.is_err() {
//                         break;
//                     }
//                 }
//                 Err(_) => break,
//             }
//         }
//     };
//
//     // Container to client forwarding
//     let container_to_client = async move {
//         let mut buffer = [0; 4096];
//         loop {
//             match container_read.read(&mut buffer).await {
//                 Ok(0) => {
//                     debug!("{} container disconnected", service_container);
//                     break;
//                 }
//                 Ok(n) => {
//                     debug!("{} forwarding {} bytes to client", service_container, n);
//                     if client_write.write_all(&buffer[..n]).await.is_err() {
//                         break;
//                     }
//                 }
//                 Err(_) => break,
//             }
//         }
//     };
//
//     // Run both directions concurrently
//     tokio::select! {
//         _ = client_to_container => {},
//         _ = container_to_client => {},
//     }
//
//     info!("{} connection closed", service);
// }
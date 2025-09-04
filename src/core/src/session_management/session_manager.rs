use crate::active_session::ActiveSession;
use crate::configuration::types::ServiceConfig;
use crate::container_management::container_manager::ContainerManager;
use crate::container_management::ContainerHandle;
use crate::data_capture::StreamRecorder;
use crate::error_handling::types::SessionError;
use crate::network::types::SessionRequest;
use crate::session::Session;
use crate::storage::storage_trait::Storage;
use crate::SessionStatus;
use chrono::Utc;
use log::error;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
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
    container_manager: Arc<Mutex<ContainerManager>>,
    storage: Arc<dyn Storage + Send + Sync>,
    max_sessions: usize,
    session_timeout: Duration,
}

impl SessionManager {
    pub fn new(
        container_manager: Arc<Mutex<ContainerManager>>,
        storage: Arc<dyn Storage + Send + Sync>,
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
        let request_stream = request.stream.take().ok_or(SessionError::CreationFailed)?;

        if let Some(active_session) = self.find_session(&request) {
            let container_tcp_socket = active_session
                .container_handle
                .as_mut()
                .and_then(|handle| handle.tcp_socket.take())
                .ok_or(SessionError::CreationFailed)?;

            active_session
                .stream_recorder
                .start_tcp_proxy(request_stream, container_tcp_socket)
                .await
                .map_err(|_| {
                    error!("An error occurred in data_capture");
                    SessionError::CreationFailed
                })?;

            return Ok(());
        }

        let (session, container_handle) = self.create_session(request, service_config).await?;
        let id = session.id;

        let mut active_session = ActiveSession {
            session,
            container_handle: Some(container_handle),
            stream_recorder: StreamRecorder::new(id, self.storage.clone()),
        };

        let container_tcp_socket = active_session
            .container_handle
            .as_mut()
            .and_then(|handle| handle.tcp_socket.take())
            .ok_or(SessionError::CreationFailed)?;

        active_session
            .stream_recorder
            .start_tcp_proxy(request_stream, container_tcp_socket)
            .await
            .map_err(|_| {
                error!("An error occurred in data_capture");
                SessionError::CreationFailed
            })?;

        self.active_sessions.insert(id, active_session);

        Ok(())
    }

    fn find_session(&mut self, request: &SessionRequest) -> Option<&mut ActiveSession> {
        let found = self.active_sessions.iter_mut().find(|(_, active_s)| {
            request.client_addr.ip() == active_s.session.client_addr.ip()
                && request.client_addr.port() == active_s.session.client_addr.port()
        });
        match found {
            Some((_, active_s)) => Some(active_s),
            None => None,
        }
    }

    pub async fn cleanup_expired_sessions(&mut self) {
        let now = Utc::now();
        let timeout_secs = self.session_timeout.as_secs() as i64;

        let mut expired = Vec::new();

        self.active_sessions.retain(|id, session| {
            let expired_session = session
                .session
                .end_time
                .map(|end| (now - end).num_seconds() >= timeout_secs)
                .unwrap_or(false);

            if expired_session {
                expired.push((id.clone(), session.container_handle.take()));
                session.session.status = SessionStatus::Completed;
                false // remove from map
            } else {
                true // keep in map
            }
        });

        for (id, handle_opt) in expired {
            if let Some(container_handle) = handle_opt {
                let mut manager = self.container_manager.lock().await;
                if let Err(e) = manager.cleanup_container(container_handle).await {
                    error!("failed to clean up container for {}: {:?}", id, e);
                }
            }
        }
    }

    pub async fn shutdown_all_sessions(&mut self) -> Result<(), SessionError> {
        for active_session in self.active_sessions.values_mut() {
            active_session.session.status = SessionStatus::Completed;

            if let Some(container_handle) = active_session.container_handle.take() {
                let mut manager = self.container_manager.lock().await;
                manager
                    .cleanup_container(container_handle)
                    .await
                    .map_err(SessionError::ContainerError)?;
            }
        }
        Ok(())
    }

    async fn create_session(
        &mut self,
        request: SessionRequest,
        service_config: &ServiceConfig,
    ) -> Result<(Session, ContainerHandle), SessionError> {
        if self.max_sessions == self.active_sessions.len() {
            return Err(SessionError::CreationFailed);
        }

        let container_handle = match self
            .container_manager
            .lock()
            .await
            .create_container(service_config)
            .await
        {
            Ok(container_handle) => container_handle,
            Err(e) => {
                error!("Failed to create container: {:?}", e);
                return Err(SessionError::CreationFailed);
            }
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

        Ok((new_session, container_handle))
    }
}

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
use log::{error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

/// The structure related to session management
///
/// This structure allow to manage session requests linked to an incoming connection
/// from network's modules.
pub struct SessionManager {
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

            // Start TCP proxy with existing session's recorder
            {
                let recorder = active_session.stream_recorder.lock().await;
                recorder
                    .start_tcp_proxy(request_stream, container_tcp_socket)
                    .await
                    .map_err(|e| {
                        error!("TCP proxy failed for session {}: {:?}", active_session.session.id, e);
                        SessionError::CreationFailed
                    })?;
            }

            // Attempt to start stdio capture if PTY is available
            if let Some(ref container_handle) = active_session.container_handle {
                if let Some(pty_master) = container_handle.pty_master.as_ref() {
                    let mut recorder = active_session.stream_recorder.lock().await;
                    if let Err(e) = recorder.start_stdio_capture(pty_master.try_clone().map_err(|_| SessionError::CreationFailed)?) {
                        warn!("Failed to start stdio capture for session {}: {:?}", active_session.session.id, e);
                        // Continue execution - stdio capture is optional
                    }
                }
            }

            return Ok(());
        }

        let (session, container_handle) = self.create_session(request, service_config).await?;
        let id = session.id;

        let mut active_session = ActiveSession {
            session,
            container_handle: Some(container_handle),
            stream_recorder: Arc::new(Mutex::new(StreamRecorder::new(id, self.storage.clone()))),
        };

        let container_tcp_socket = active_session
            .container_handle
            .as_mut()
            .and_then(|handle| handle.tcp_socket.take())
            .ok_or(SessionError::CreationFailed)?;

        // Start TCP proxy for new session
        {
            let recorder = active_session.stream_recorder.lock().await;
            recorder
                .start_tcp_proxy(request_stream, container_tcp_socket)
                .await
                .map_err(|e| {
                    error!("TCP proxy failed for new session {}: {:?}", id, e);
                    SessionError::CreationFailed
                })?;
        }

        // Attempt to start stdio capture if PTY is available
        if let Some(ref container_handle) = active_session.container_handle {
            if let Some(pty_master) = container_handle.pty_master.as_ref() {
                let mut recorder = active_session.stream_recorder.lock().await;
                if let Err(e) = recorder.start_stdio_capture(pty_master.try_clone().map_err(|_| SessionError::CreationFailed)?) {
                    warn!("Failed to start stdio capture for new session {}: {:?}", id, e);
                    // Continue execution - stdio capture is optional
                }
            }
        }

        self.active_sessions.insert(id, active_session);
        info!("Created new session {} with capture lifecycle initialized", id);

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
                expired.push(*id);
                false // remove from map
            } else {
                true // keep in map
            }
        });

        // Finalize captures and cleanup containers for expired sessions
        for session_id in expired {
            if let Err(e) = self.end_session(&session_id).await {
                error!("Failed to clean up expired session {}: {:?}", session_id, e);
            }
        }
    }

    pub async fn shutdown_all_sessions(&mut self) -> Result<(), SessionError> {
        let session_ids: Vec<Uuid> = self.active_sessions.keys().cloned().collect();

        // End all sessions, which will finalize captures and cleanup containers
        for session_id in session_ids {
            if let Err(e) = self.end_session(&session_id).await {
                error!("Failed to shutdown session {}: {:?}", session_id, e);
            }
        }

        self.active_sessions.clear();
        Ok(())
    }

    /// Finalizes the capture for a specific session and persists the artifacts
    pub async fn finalize_session_capture(&mut self, session_id: &Uuid) -> Result<(), SessionError> {
        if let Some(active_session) = self.active_sessions.get_mut(session_id) {
            active_session.session.end_time = Some(Utc::now());

            let recorder = active_session.stream_recorder.lock().await;
            match recorder.finalize_capture() {
                Ok(artifacts) => {
                    info!("Successfully finalized capture for session {}: {} total bytes captured",
                          session_id, artifacts.total_bytes);

                    // Update session with capture statistics
                    active_session.session.bytes_transferred = artifacts.total_bytes;
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to finalize capture for session {}: {:?}", session_id, e);
                    active_session.session.status = SessionStatus::Error;
                    Err(SessionError::CaptureError(e))
                }
            }
        } else {
            warn!("Attempted to finalize capture for non-existent session: {}", session_id);
            Err(SessionError::NotFound)
        }
    }

    /// Manually end a session and finalize its capture
    pub async fn end_session(&mut self, session_id: &Uuid) -> Result<(), SessionError> {
        if let Some(mut active_session) = self.active_sessions.remove(session_id) {
            // Finalize capture
            active_session.session.end_time = Some(Utc::now());
            let recorder = active_session.stream_recorder.lock().await;

            match recorder.finalize_capture() {
                Ok(artifacts) => {
                    info!("Successfully finalized capture for session {}: {} total bytes captured",
                          session_id, artifacts.total_bytes);
                    active_session.session.bytes_transferred = artifacts.total_bytes;
                }
                Err(e) => {
                    error!("Failed to finalize capture for session {}: {:?}", session_id, e);
                    active_session.session.status = SessionStatus::Error;
                }
            }

            active_session.session.status = SessionStatus::Completed;

            // Clean up container if present
            if let Some(container_handle) = active_session.container_handle.take() {
                let mut manager = self.container_manager.lock().await;
                manager.cleanup_container(container_handle).await
                    .map_err(SessionError::ContainerError)?;
            }

            info!("Session {} ended and cleaned up successfully", session_id);
            Ok(())
        } else {
            Err(SessionError::NotFound)
        }
    }

    /// Get access to a session's stream recorder for additional capture operations
    pub fn get_session_recorder(&self, session_id: &Uuid) -> Option<Arc<Mutex<StreamRecorder>>> {
        self.active_sessions.get(session_id).map(|session| session.stream_recorder.clone())
    }

    /// Trigger stdio capture for a specific session if PTY is available
    pub async fn trigger_stdio_capture(&mut self, session_id: &Uuid) -> Result<(), SessionError> {
        if let Some(active_session) = self.active_sessions.get_mut(session_id) {
            if let Some(ref container_handle) = active_session.container_handle {
                if let Some(pty_master) = container_handle.pty_master.as_ref() {
                    let mut recorder = active_session.stream_recorder.lock().await;
                    recorder.start_stdio_capture(pty_master.try_clone().map_err(|_| SessionError::CreationFailed)?)
                        .map_err(SessionError::CaptureError)?;
                    info!("Manually triggered stdio capture for session {}", session_id);
                    Ok(())
                } else {
                    warn!("No PTY available for session {}", session_id);
                    Err(SessionError::CreationFailed)
                }
            } else {
                warn!("No container handle available for session {}", session_id);
                Err(SessionError::CreationFailed)
            }
        } else {
            Err(SessionError::NotFound)
        }
    }

    /// Get session statistics including capture information
    pub fn get_session_stats(&self, session_id: &Uuid) -> Option<(SessionStatus, u64, chrono::Duration)> {
        self.active_sessions.get(session_id).map(|session| {
            let duration = if let Some(end_time) = session.session.end_time {
                end_time - session.session.start_time
            } else {
                Utc::now() - session.session.start_time
            };
            (session.session.status.clone(), session.session.bytes_transferred, duration)
        })
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

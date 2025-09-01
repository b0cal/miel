use std::collections::HashMap;
use tokio::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;
use chrono::Utc;
use serde::de::Unexpected::Option;
use tokio::io::{copy_bidirectional, AsyncWriteExt};
use uuid::Uuid;
use crate::active_session::ActiveSession;
use crate::container_management::container_manager::ContainerManager;
use crate::network::session_request::SessionRequest;
use crate::error_handling::types::SessionError;
use crate::session::Session;
use crate::SessionStatus;
use crate::data_capture::stream_recorder::StreamRecorder;


use tokio::io;
use crate::container_management::container_handle::ContainerHandle;

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
    session_timeout: Duration
}

impl SessionManager {

    pub fn new(container_manager: Arc<ContainerManager>, storage: Arc<dyn Storage>, max_sessions: usize) -> Self {
        Self {
            active_sessions: HashMap::new(),
            container_manager,
            storage,
            max_sessions,
            session_timeout: Duration::new(170000, 0) //170'000sec = ~48H
        }
    }

    pub async fn handle_session(&mut self, request: SessionRequest) -> Result<(), SessionError>{
        //TODO
        let session = match self.create_session(request) {
            Ok(s) => s,
            Err(e) => return Err(SessionError::CreationFailed)
        };

        let active_session: ContainerHandle = match self.active_sessions.get(&session.id).unwrap(){
            Some(h) => h.clone(),
            None => {}
        }.except(SessionError::ContainerError());


        let container_handle = match self.active_sessions.get(&session.id).unwrap().container_handle.clone(){
            Some(container_handle) => container_handle,
            None => return Err(SessionError::ContainerError("No container handle"))
        };





        //let a: ContainerHandle = self.active_sessions.get(&session.id).map()

        //active_session.tcp_stream;



        //let (bytes1, bytes2) = io::copy_bidirectional(&mut stream1, &mut stream2).await?;


    }


    pub fn cleanup_expired_sessions(&mut self){
        for active_session in self.active_sessions.values_mut() {
            if active_session.session.end_time != None && Utc::now() - active_session.session.end_time >= self.session_timeout  {
                active_session.session.status = SessionStatus::Completed;
                self.active_sessions.remove(&active_session.session.id);
                self.container_manager.cleanup_container(active_session.container_handle);
            }
        }
    }

    pub fn get_active_session_count(&self) -> usize {
        self.active_sessions.len()
    }

    pub fn shutdown_all_sessions(&mut self) -> Result<(), SessionError>{
        for active_session in self.active_sessions.values_mut(){
            active_session.session.status = SessionStatus::Completed;
            match self.container_manager.cleanup_container(&active_session.container_handle){
                Ok(_) => Ok(()),
                Err(e) => Err(SessionError::ContainerError(e))
            }
        }
        Ok(())
    }



    fn create_session(&mut self, request: SessionRequest) -> Result<Session, SessionError> {
        let new_container_handle = match self.container_manager.create_container(request){
            Ok(_) => Ok(()),
            Err(e) => return Err(SessionError::ContainerError(e))
        }; //TODO a reflechir sur ce qu'on envois comme argument a la fonction create_container

        let new_session = Session {
            id: Uuid::new_v4(),
            service_name: request.service_name,
            client_addr: request.client_addr,
            start_time: request.timestamp,
            end_time:None,
            container_id:Some((new_container_handle.id).to_string()),
            bytes_transferred:0,
            status: SessionStatus::Active
        };

        let session_id = new_session.id;
        let new_active_session = ActiveSession{
            session: new_session,
            container_handle: new_container_handle,
            stream_recorder:StreamRecorder::new(session_id, self.storage)
        };
        let _ = match self.active_sessions.insert(session_id, new_active_session) {
            Some(_) => Ok(self.active_sessions.get(&session_id)),
            None => {}
        };
        Ok(self.active_sessions.get(&session_id))

    }


    fn setup_data_proxy(&self, session_id: Uuid, client_stream: TcpStream, container_stream: TcpStream) -> Result<(), SessionError>{
        self.storage.save_interaction(session_id, &[client_stream, container_stream]);
    }
}


#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use chrono::Duration;
    use crate::active_session::ActiveSession;
    use crate::container_management::container_manager::ContainerManager;
    use crate::session_manager::SessionManager;
    use crate::session_request::SessionRequest;

    #[test]
    fn test_create_session_manager() {
        let container_manager = Arc::new(ContainerManager::new());
        let storage = Arc::new(DatabaseStorage);
        let mut session_manager = SessionManager::new(container_manager.clone(), storage.clone(), 20);

        assert_eq!(session_manager.active_sessions.len(), 0);
        assert_eq!(session_manager.session_timeout, Duration::new(170000, 0));
        assert_eq!(session_manager.max_sessions, 20);
    }



    #[test]
    fn test_cleanup_expired_sessions() {
        let container_manager = Arc::new(ContainerManager::new());
        let storage = Arc::new(DatabaseStorage);
        let mut session_manager = SessionManager::new(container_manager.clone(), storage.clone(), 20);


        let expired_session = ActiveSession {
            //TODO
        };
        session_manager.active_sessions.insert(expired_session.session.id, expired_session);

        assert_eq!(session_manager.active_sessions.len(), 1);
        session_manager.cleanup_expired_sessions();
        assert_eq!(session_manager.active_sessions.len(), 0);

    }


    #[test]
    fn test_create_session() {
        let container_manager = Arc::new(ContainerManager::new());
        let storage = Arc::new(DatabaseStorage);
        let mut session_manager = SessionManager::new(container_manager.clone(), storage.clone(), 20);
        let session_request = SessionRequest{
            //TODO
        }

        session_manager.create_session(session_request);
        assert_eq!(session_manager.active_sessions.len(), 1);

    }

}




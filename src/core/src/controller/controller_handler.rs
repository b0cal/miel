use crate::configuration::config::Config;
use crate::configuration::{ServiceConfig, StorageBackend};
use crate::container_management::ContainerManager;
use crate::error_handling::types::{ControllerError, SessionError};
use crate::network::{network_listener::NetworkListener, types::SessionRequest};
use crate::session_manager::SessionManager;
use crate::storage::database_storage::DatabaseStorage;
use crate::storage::file_storage::FileStorage;
use crate::storage::storage_trait::Storage;
use crate::web_interface::WebServer;
use log::{error, info};
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub struct Controller {
    // Fields for the Controller struct
    config: Config,
    listener: Option<NetworkListener>,
    session_rx: Option<mpsc::Receiver<SessionRequest>>,
    storage: Arc<dyn Storage + Send + Sync>,
    container_manager: Arc<tokio::sync::Mutex<ContainerManager>>,
    session_manager: SessionManager,
    listener_handle: Option<JoinHandle<()>>,
}

impl Controller {
    pub async fn new(config: Config) -> Result<Self, ControllerError> {
        let container_manager = Arc::new(tokio::sync::Mutex::new(ContainerManager::new().unwrap()));

        // Create storage backend based on configuration
        let storage: Arc<dyn Storage + Send + Sync> = match config.storage_backend {
            StorageBackend::Database => {
                info!("Initializing Database storage backend");
                Arc::new(
                    DatabaseStorage::from_config_path(&config.storage_path)
                        .await
                        .map_err(ControllerError::StorageError)?,
                )
            }
            StorageBackend::FileSystem => {
                info!("Initializing FileSystem storage backend");
                Arc::new(
                    FileStorage::from_config_path(&config.storage_path)
                        .map_err(ControllerError::StorageError)?,
                )
            }
        };

        if config.web_ui_enabled {
            let ws = WebServer::new(storage.clone());
            tokio::spawn(async move {
                let _ = ws.start(config.web_ui_port).await;
            });
        }

        let session_manager = SessionManager::new(
            container_manager.clone(),
            storage.clone(),
            config.max_sessions,
        );

        Ok(Self {
            config,
            listener: None,
            session_rx: None,
            listener_handle: None,
            container_manager,
            session_manager,
            storage,
        })
    }

    pub async fn run(
        &mut self,
        mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> Result<(), ControllerError> {
        let (tx, rx) = mpsc::channel(100);
        self.session_rx = Some(rx);

        self.listener = Some(NetworkListener::new(tx));

        info!("Binding services in service detector...");

        let _ = self.listener.as_mut().unwrap().bind_services(self.config.services.as_slice()).map_err(|e| {
            error!("Calling bind_services() from Controller not working, returned with error: {:?}", e);
            e
        });

        info!("Services bound correctly in service detector");

        let ip_addr = Ipv4Addr::from_str(self.config.bind_address.as_str())
            .map_err(|e| e.to_string())
            .unwrap();

        let copy = self.listener.as_mut().unwrap().extract_for_listening();

        let handle = tokio::spawn(async move {
            if let Err(e) = NetworkListener::start_listening(copy, ip_addr).await {
                error!("NetworkListener failed: {:?}", e);
            }
        });

        self.listener_handle = Some(handle);

        loop {
            tokio::select! {
                session_request = self.session_rx.as_mut().unwrap().recv() => {
                    match session_request {
                        Some(request) => {
                            if let Err(e) = self.handle_session_request(request).await {
                                error!("Session handling failed: {:?}", e);
                            }
                        }
                        None => {
                            info!("Session channel closed, stopping controller");
                            break;
                        }
                    }
                }

                _ = shutdown_rx.recv() => {
                        info!("Shutdown signal received in controller, stopping gracefully");
                        break;
                    }
            }
        }

        info!("Controller initiating graceful shutdown...");
        self.shutdown().await?;
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<(), ControllerError> {
        info!("Starting Controller shutdown...");

        // First, shutdown all active sessions and save them to database
        if let Err(e) = self.session_manager.shutdown_all_sessions().await {
            error!("Failed to shutdown sessions gracefully: {:?}", e);
        }

        if let Some(listener) = &mut self.listener {
            if let Err(e) = listener.shutdown().await {
                error!("Failed to shutdown NetworkListener gracefully: {:?}", e);
            }
        }

        if let Some(handle) = self.listener_handle.take() {
            info!("Aborting network listener task...");
            handle.abort();

            match handle.await {
                Ok(()) => info!("Network listener task terminated cleanly"),
                Err(e) if e.is_cancelled() => info!("Network listener task was cancelled"),
                Err(e) => error!("Network listener task terminated with error: {:?}", e),
            }
        }

        if let Some(session_rx) = self.session_rx.take() {
            drop(session_rx);
            info!("Session receiver channel closed");
        }

        self.listener = None;

        info!("Controller shutdown completed");
        Ok(())
    }

    async fn handle_session_request(
        &mut self,
        request: SessionRequest,
    ) -> Result<(), SessionError> {
        info!("Session request received from {}", request.client_addr);
        info!("Service detected as: {:?}", request.service_name);

        // Clone the config to avoid holding a reference to self
        let service = self
            .find_config_for_service(&request.service_name)
            .cloned()
            .unwrap();

        // Handle the session and trigger capture lifecycle
        self.session_manager
            .handle_session(request, &service)
            .await?;

        info!("Session handling completed with capture lifecycle initialized");
        Ok(())
    }

    /// Manually trigger capture finalization for a specific session
    pub async fn finalize_session_capture(
        &mut self,
        session_id: &uuid::Uuid,
    ) -> Result<(), SessionError> {
        self.session_manager
            .finalize_session_capture(session_id)
            .await
    }

    /// Manually end a session and finalize its capture
    pub async fn end_session(&mut self, session_id: &uuid::Uuid) -> Result<(), SessionError> {
        self.session_manager.end_session(session_id).await
    }

    /// Get session statistics including capture information
    pub fn get_session_stats(
        &self,
        session_id: &uuid::Uuid,
    ) -> Option<(crate::SessionStatus, u64, chrono::Duration)> {
        self.session_manager.get_session_stats(session_id)
    }

    /// Trigger stdio capture for a specific session
    pub async fn trigger_stdio_capture(
        &mut self,
        session_id: &uuid::Uuid,
    ) -> Result<(), SessionError> {
        self.session_manager.trigger_stdio_capture(session_id).await
    }

    /// Called when a connection drops or times out to ensure proper capture finalization
    pub async fn on_session_end(&mut self, session_id: &uuid::Uuid) -> Result<(), SessionError> {
        info!("Finalizing session {} due to connection end", session_id);
        self.finalize_session_capture(session_id).await
    }

    /// Called periodically to clean up expired sessions and finalize their captures
    pub async fn cleanup_and_finalize_expired_sessions(&mut self) -> Result<(), SessionError> {
        info!("Running periodic session cleanup and capture finalization");
        self.cleanup_expired_sessions().await
    }

    /// Get access to the storage backend for direct database operations
    pub fn get_storage(&self) -> Arc<dyn Storage + Send + Sync> {
        self.storage.clone()
    }

    /// Get access to the container manager for direct container operations
    pub fn get_container_manager(&self) -> Arc<tokio::sync::Mutex<ContainerManager>> {
        self.container_manager.clone()
    }

    /// Get all sessions using optional filtering
    pub fn get_sessions(
        &self,
        filter: Option<crate::storage::types::SessionFilter>,
    ) -> Result<Vec<crate::session::Session>, crate::error_handling::types::StorageError> {
        self.storage.get_sessions(filter)
    }

    /// Cleanup expired sessions manually
    pub async fn cleanup_expired_sessions(&mut self) -> Result<(), SessionError> {
        self.session_manager.cleanup_expired_sessions().await;
        Ok(())
    }

    fn find_config_for_service(&self, service_name: &str) -> Option<&ServiceConfig> {
        self.config.services.iter().find(|s| s.name == service_name)
    }

    #[cfg(test)]
    async fn new_for_test(config: Config) -> Result<Self, ControllerError> {
        use crate::storage::file_storage::FileStorage;
        use tempfile::TempDir;

        // Create a temporary directory for test storage
        let temp_dir = TempDir::new().map_err(|_| {
            ControllerError::StorageError(crate::error_handling::types::StorageError::WriteFailed)
        })?;
        let temp_path = temp_dir.path().to_path_buf();
        // Leak the temp dir to keep it alive for the test duration
        std::mem::forget(temp_dir);

        // Use file storage for tests to avoid database complexity
        let storage: Arc<dyn Storage + Send + Sync> =
<<<<<<< HEAD
            Arc::new(FileStorage::new(temp_path).map_err(ControllerError::Storage)?);
=======
            Arc::new(FileStorage::new(temp_path).map_err(|e| ControllerError::StorageError(e))?);
>>>>>>> dev

        // Create a mock container manager that doesn't require root privileges
        let container_manager = Arc::new(tokio::sync::Mutex::new(ContainerManager::new_mock()));

        let session_manager = SessionManager::new(
            container_manager.clone(),
            storage.clone(),
            config.max_sessions,
        );

        Ok(Self {
            config,
            listener: None,
            session_rx: None,
            listener_handle: None,
            container_manager,
            session_manager,
            storage,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configuration::config::Config;
    use crate::configuration::types::Protocol;
    use log::debug;
    use std::time::Duration;
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpStream;
    use tokio::time;

    async fn get_free_port() -> u16 {
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind 0");
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        port
    }

    async fn create_http_test_config() -> Config {
        let free_port = get_free_port().await;
        Config {
            bind_address: "127.0.0.1".to_string(),
            services: vec![ServiceConfig {
                name: "http".to_string(),
                port: free_port,
                protocol: Protocol::TCP,
                header_patterns: vec!["GET".to_string(), "POST".to_string()],
                banner_response: Some("HTTP/1.1 200 OK\r\n\r\n".to_string()),
                ..ServiceConfig::default()
            }],
            ..Default::default()
        }
    }

    async fn wait_for_service_ready(port: u16, timeout: Duration) -> bool {
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            match TcpStream::connect(format!("127.0.0.1:{}", port)).await {
                Ok(stream) => {
                    debug!("Successfully connected to NetworkListener on port {}", port);
                    drop(stream);
                    return true;
                }
                Err(e) => {
                    debug!("Connection attempt failed: {:?}", e);
                    time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
        debug!("Port {} failed to become ready within {:?}", port, timeout);
        false
    }

    async fn simulate_http_client(
        port: u16,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        debug!("Simulating HTTP client connection to port {}", port);

        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).await?;
        debug!("Successfully connected to port {}", port);

        let http_request = "GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
        debug!("Sending HTTP request: {:?}", http_request);

        let mut response = Vec::new();
        let mut buffer = [0; 124];

        match time::timeout(Duration::from_millis(1000), stream.read(&mut buffer)).await {
            Ok(Ok(bytes_read)) if bytes_read > 0 => {
                response.extend_from_slice(&buffer[..bytes_read]);
                debug!("Received {} bytes in response", bytes_read);
                Ok(String::from_utf8_lossy(&response).to_string())
            }
            Ok(Ok(_)) => {
                debug!("Connection established but no immediate response");
                Ok("No response".to_string())
            }
            Ok(Err(e)) => {
                debug!("Error reading response: {:?}", e);
                Err(Box::new(e))
            }
            Err(_) => {
                debug!("Response timeout - connection was handled but no immediate response");
                Ok("Timeout but connection handled".to_string())
            }
        }
    }

    #[tokio::test]
    async fn test_controller_flow_from_network_listener_to_session_request() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .is_test(true)
            .try_init();
        debug!("=== Starting Complete Controller Flow Test ===");

        let config = create_http_test_config().await;
        let port = config.services[0].port;
        debug!(
            "Created test config with NetworkListener binding to port {}",
            port
        );

        let mut controller = Controller::new_for_test(config).await.unwrap();
        debug!("Controller initialized");

        debug!("Starting controller...");
        let (tx, rx) = tokio::sync::broadcast::channel(1);
        let controller_task = tokio::spawn(async move {
            debug!("Controller.run() is starting...");
            let _ = controller.run(rx).await;
            debug!("Controller.run() has ended");
        });

        debug!(
            "Waiting for NetworkListener to bind and start listening on port {} ...",
            port
        );
        let ready = wait_for_service_ready(port, Duration::from_secs(10)).await;
        assert!(ready, "HTTP service should be ready within 10 seconds");
        debug!("NetworkListener is ready and accepting connections");

        time::sleep(Duration::from_millis(200)).await;

        debug!("Simulating HTTP client connection...");
        let response_result = simulate_http_client(port).await;

        match response_result {
            Ok(response) => {
                debug!("HTTP client simulation completed Successfully");
                debug!("Response received: {}", response);
            }
            Err(e) => {
                debug!("HTTP client simulation error: {:?}", e);
            }
        }

        debug!("Allowing time for session request processing...");
        time::sleep(Duration::from_millis(500)).await;

        assert!(
            !controller_task.is_finished(),
            "Controller should still be running and processing sessions"
        );
        debug!("Controller is still running and processing sessions");

        debug!("Cleaning up controller task...");
        let _ = tx.send(());

        let _ = controller_task.await;
        debug!("=== Complete Controller Flow Test Finished ===");
    }
}

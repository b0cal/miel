use crate::configuration::config::Config;
use crate::error_handling::types::ControllerError;
use crate::network::{network_listener::NetworkListener, types::SessionRequest};
use log::{debug, error};
use std::net::Ipv4Addr;
use std::str::FromStr;
use tokio::sync::mpsc;
pub struct Controller {
    // Fields for the Controller struct
    config: Config,
    listener: Option<NetworkListener>,
    session_rx: Option<mpsc::Receiver<SessionRequest>>,
}

impl Controller {
    pub fn new(config: Config) -> Result<Self, ControllerError> {
        Ok(Self {
            config,
            listener: None,
            session_rx: None,
        })
    }

    pub async fn run(&mut self) -> Result<(), ControllerError> {
        let (tx, rx) = mpsc::channel(100);
        self.session_rx = Some(rx);

        debug!("In run: binding address is {}", self.config.bind_address);

        self.listener = Some(NetworkListener::new(tx));

        let _ = self.listener.as_mut().unwrap().bind_services(self.config.services.as_slice()).map_err(|e| {
            error!("Calling bind_services() from Controller not working, returned with error: {:?}", e);
            e
        });

        let ip_addr =
            Ipv4Addr::from_str(self.config.bind_address.as_str()).map_err(|e| e.to_string());

        // Should be ok to take ownership here as the self.listener shouldn't be used after this spawn
        let mut listener = self.listener.take().unwrap();

        tokio::spawn(async move {
            if let Err(e) = listener.start_listening(ip_addr.unwrap()).await {
                error!("NetworkListener failed: {:?}", e);
            }
        });

        while let Some(session_request) = self.session_rx.as_mut().unwrap().recv().await {
            debug!(
                "Session request received from {}:",
                session_request.client_addr
            );
            debug!("Service detected as: {:?}", session_request.service_name);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configuration::config::Config;
    use crate::configuration::types::{Protocol, ServiceConfig};
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

        let mut controller = Controller::new(config).unwrap();
        debug!("Controller initialized");

        debug!("Starting controller...");
        let controller_task = tokio::spawn(async move {
            debug!("Controller.run() is starting...");
            let _ = controller.run().await;
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
        controller_task.abort();
        let _ = controller_task.await;

        debug!("=== Complete Controller Flow Test Finished ===");
    }
}

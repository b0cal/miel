//! # Network Listener Module
//!
//! This module provides network listening capacities for handling incoming TCP connections and
//! routing them to appropriate service based on detected protocol
//!
//! The main component is [`NetworkListener`] which manages multiple TCP sockets, detects incoming
//! service types, filters connections, and forwards valid sessions to the [`SessionManager`] via
//! [`SessionRequest`] through an async channel.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
//! │ Incoming        │───▶│ NetworkListener  │───▶│ SessionManager  │
//! │ Connections     │    │                  │    │ (via mpsc)      │
//! └─────────────────┘    │ - Service Detection   └─────────────────┘
//!                        │ - Connection Filter
//!                        │ - Protocol Analysis
//!                        └──────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,no_run
//! use tokio::sync::mpsc;
//! use miel::configuration::types::ServiceConfig;
//! use miel::network::network_listener::NetworkListener;
//! use miel::error_handling::types::NetworkError;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), NetworkError> {
//!     // Create a channel for session requests
//!     let (tx, rx) = mpsc::channel(100);
//!     
//!     // Initialize the network listener
//!     let mut listener = NetworkListener::new(tx);
//!     
//!     // Configure services to listen on
//!     let services = vec![
//!         ServiceConfig { port: 8080, ..Default::default() },
//!         ServiceConfig { port: 8443, ..Default::default() },
//!     ];
//!
//!     // Bind to the configured service
//!     listener.bind_services(&services)?;
//!
//!     // Start listening for connections
//!     listener.start_listening().await?;
//!
//!     Ok(())
//! }
//! ```

use super::connection_filter::*;
use super::service_detector::*;
use super::session_request::*;
use crate::configuration::types::ServiceConfig;
use crate::error_handling::types::NetworkError;
use chrono::Utc;
use log::error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_test::io::{Builder, Mock};

use std::collections::HashMap;
use tokio::net::{TcpListener, TcpSocket, TcpStream};
use tokio::sync::mpsc::Sender;

/// A network listener that manages multiple TCP socket and routes connections to services.
///
/// `NetworkListener` is responsible for:
/// - Binding to multiple ports based on service configuration
/// - Detecting the type of incoming service requests
/// - Filtering connections based on security policies
/// - Forwarding valid sessions to the session manager
///
/// The listener operates asynchronously and uses an MPSC channel to communicate with the session
/// manager
///
/// # Examples
///
/// ```rust, no_run
/// use tokio::sync::mpsc;
/// use miel::network::network_listener::NetworkListener;
/// use miel::configuration::types::ServiceConfig;
///
/// #[tokio::main]
/// async fn main() {
///     let (tx, rx) = mpsc::channel(100);
///     let mut listener = NetworkListener::new(tx);
///
///     // Configure and bind services
///     let services = vec![ServiceConfig::default()];
///     listener.bind_services(&services).unwrap();
/// }
/// ```
pub struct NetworkListener {
    /// Map of port number to their corresponding TCP sockets
    listeners: HashMap<u16, TcpSocket>,

    /// Channel sender for forwarding session requests to the session manager
    session_tx: Sender<SessionRequest>,

    /// Service detection component for identifying connection protocols
    service_detector: ServiceDetector,

    /// Connection filtering component for security and access control
    connection_filter: ConnectionFilter,
}

impl NetworkListener {
    /// Creates a new `NetworkListener` instance.
    ///
    /// # Arguments
    ///
    /// * `session_tx` - A channel sender for forwarding session requests to the session manager
    ///
    /// # Returns
    ///
    /// A new `NetworkListener` with empty listeners and default components.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use tokio::sync::mpsc;
    /// use miel::network::network_listener::NetworkListener;
    ///
    /// let (tx, rx) = mpsc::channel(100);
    /// let listener = NetworkListener::new(tx);
    /// ```
    pub fn new(session_tx: Sender<SessionRequest>) -> Self {
        Self {
            listeners: HashMap::new(),
            session_tx,
            service_detector: ServiceDetector {
                service_patterns: HashMap::new(),
            },
            connection_filter: ConnectionFilter::default(),
        }
    }

    /// Binds TCP sockets to the ports specified in the service configurations.
    ///
    /// This method creates and configures TCP sockets for each service, storing them in the
    /// internal listeners map. It also initializes the service detector with the provided service
    /// configurations
    ///
    /// # Arguments
    ///
    /// * `services` - A slice of service configurations containing port and protocol information
    ///
    /// # Returns
    ///
    /// * `Ok(())` if all services were successfully bound
    /// * `Err(NetworkError::SockError)` if any socket creation fails
    ///
    /// # Error
    ///
    /// This function will return an error if:
    /// - TCP socket creation fails for any of the specified ports
    /// - The system runs out of available file descriptors
    /// - Permission is denied for binding to privileged ports (< 1024)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use tokio::mpsc;
    /// use miel::configuration::types::ServiceConfig;
    /// use miel::network::network_listener::NetworkListener;
    ///
    /// let (tx, rx) = mpsc::channel(100);
    /// let mut listener = NetworkListener::new(tx);
    ///
    /// let services = vec![
    ///     ServiceConfig { port: 8080, ..Default::default() },
    ///     ServiceConfig { port: 8443, ..Default::default() }
    /// ]
    ///
    /// match listener.bind_services(&services) {
    ///     Ok(()) => println!("All services bound successfully"),
    ///     Err(e) => eprintln!("Failed to bind services: {:?}", e),
    /// }
    /// ```
    pub fn bind_services(&mut self, services: &[ServiceConfig]) -> Result<(), NetworkError> {
        self.service_detector = ServiceDetector::new(services);

        let services_it = services.iter();

        for s in services_it {
            let socket = match TcpSocket::new_v4() {
                Ok(sock) => sock,
                Err(err) => {
                    error!("[!] Socket error: {:?}", err);
                    return Err(NetworkError::SockError(err));
                }
            };

            self.listeners.insert(s.port, socket);
        }

        Ok(())
    }

    // Not meant to stay, just so we can replicate the service for now
    async fn start_http_server() {
        let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();

        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let mut buffer = [0; 1024];
                    let _ = stream.read(&mut buffer).await;

                    let response = "HTTP/1.1 200 OK\r\n\r\nHello World!";
                    let _ = stream.write_all(response.as_bytes()).await;
                });
            }
        });
    }

    /// Starts listening for incoming connections and processes them.
    ///
    /// This method begins the main listening loop, accepting incoming connections, performing
    /// service detection, and forwarding valid sessions to the session manager
    ///
    /// The method currently includes test code :
    /// 1. Starts a test HTTP server
    /// 2. Creates a test connection
    /// 3. Sends a test session request through the channel
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the listening process starts successfully
    /// * `Err(NetworkError::ChannelFailed)` if sending through the session channel fails
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The session channel is closed or full
    /// - Network connection establishment fails
    ///
    /// # Note
    ///
    /// The current implementation is primarly for testing. In production, this method should
    /// implement the full listening loop for all configured services.
    ///
    /// # Examples
    ///
    /// ```rust,no_run`
    /// use::tokio::sync::mpsc;
    /// use miel::network::network_listener::NetworkListener;
    /// use miel::configuration::types::ServiceConfig;
    /// use miel::error_handling::types::NetworkError;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), NetworkError> {
    ///     let (tx, rx) = mpsc::channel(100);
    ///     let mut listener = NetworkListener::new(tx);
    ///
    ///     listener.bind_services(&[ServiceConfig::default()])?;
    ///     listener.start_listening().await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn start_listening(&self) -> Result<(), NetworkError> {
        // For testing purposes we need to have the TcpStream address and port available, for now
        // we use a simple HTTP server that responds '200 OK Hello World'
        Self::start_http_server().await;

        let client_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8000);

        let request = SessionRequest {
            stream: TcpStream::connect("0.0.0.0:8000").await.unwrap(),
            service_name: "test_service".to_string(),
            client_addr,
            timestamp: Utc::now(),
        };

        match self.session_tx.send(request).await {
            Ok(_) => (),
            Err(_) => {
                return Err(NetworkError::ChannelFailed);
            }
        }

        Ok(())
    }
    pub fn shutdown() -> Result<(), NetworkError> {
        Ok(())
    }
    fn handle_connection(stream: TcpStream, service: &str) {}

    fn create_mock_stream() -> Mock {
        Builder::new().read(b"hello").write(b"world").build()
    }

    fn create_session_request() -> SessionRequest<tokio_test::io::Mock> {
        let mock_stream = Builder::new().read(b"hello").write(b"world").build();
        let client_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 45)), 12345);

        SessionRequest {
            stream: mock_stream,
            service_name: String::new(),
            client_addr,
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use tokio::sync::mpsc;
    use tokio_test::io::{Builder, Mock};

    #[test]
    fn test_bind_services_from_legit_file() {
        // Build channel
        let (tx, mut rx) = mpsc::channel(100);

        // Create new NetworkListener
        let mut listener = NetworkListener::new(tx);

        // Binding a server shouldn't return an error, if returns Err, panic! and test fails
        listener.bind_services(&[ServiceConfig::default()]).unwrap();
    }

    #[tokio::test]
    async fn test_start_listening_channel_communication() {
        let (tx, mut rx) = mpsc::channel(100);

        let mut listener = NetworkListener::new(tx);
        listener.bind_services(&[ServiceConfig::default()]).unwrap();

        listener.start_listening().await.unwrap();

        let received = rx.recv().await.unwrap();

        assert_eq!(received.service_name, "test_service".to_string());
    }
}

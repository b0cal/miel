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
use super::types::SessionRequest;
use crate::configuration::types::ServiceConfig;
use crate::error_handling::types::NetworkError;

use chrono::Utc;
use log::error;
use std::collections::HashMap;
use std::net::SocketAddr;
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
    /*
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
    */

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
    pub async fn start_listening(&mut self) -> Result<(), NetworkError> {
        // Will handle waiting all listeners tasks to complete
        let mut listener_handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();

        // Bind all sockets and create listeners
        for (port, socket) in self.listeners.drain() {
            //TODO: Not sure of what should be the address to bind to
            let addr = SocketAddr::from(([0, 0, 0, 0], port));

            // Bind socket to the address
            if let Err(e) = socket.bind(addr) {
                error!("[!] Failed to bind to port {}: {:?}", port, e);
                return Err(NetworkError::BindError(e));
            }

            //TODO: to discuss but could it be a good idea to have associated functions in the
            //Controller to get any of the configuration value (i.e. max_connections for this part
            //of the function)

            // Convert to listener
            let listener = match socket.listen(1024) {
                // backlog value should
                // correspond to max_connection from controller
                Ok(listener) => listener,
                Err(e) => {
                    error!("[!] Failed to listen on port {}: {:?}", port, e);
                    return Err(NetworkError::BindError(e));
                }
            };

            log::info!("[+] Successfully bound to port {}", port);

            // TODO: Implement Clone trait on service_detector and connection_filter
            //
            // Clone components used for the async listening session
            let session_tx = self.session_tx.clone();
            let service_detector = self.service_detector.clone();
            let connection_filter = self.connection_filter.clone();

            let handle = tokio::spawn(async move {
                Self::listen_on_port(
                    listener,
                    session_tx,
                    service_detector,
                    connection_filter,
                    port,
                )
                .await
            });

            listener_handles.push(handle);
        }

        //Wait for all listener tasks to complete
        for handle in listener_handles {
            if let Err(e) = handle.await {
                error!("[!] Listener task panicked: {:?}", e);
            }
        }
        // For testing purposes we need to have the TcpStream address and port available, for now
        // we use a simple HTTP server that responds '200 OK Hello World'
        Ok(())
    }

    async fn listen_on_port(
        listener: TcpListener,
        session_tx: Sender<SessionRequest>,
        service_detector: ServiceDetector,
        connection_filter: ConnectionFilter,
        port: u16,
    ) {
        log::info!("[+] Started listening on port {}", port);

        loop {
            //Accept incomming connection
            let (stream, client_addr) = match listener.accept().await {
                Ok((stream, addr)) => (stream, addr),
                Err(e) => {
                    error!("[!] Failed to accept connection on port {}: {:?}", port, e);
                    continue;
                }
            };

            log::debug!("[+] New connection from {} on port {}", client_addr, port);

            // Check if connection should be accepted
            if !connection_filter.should_accept_connection(&client_addr.ip(), port) {
                log::warn!("[!] Connection from {} rejected by filter", client_addr);
                continue;
            }

            // Clone components for the connection handling task
            let session_tx_clone = session_tx.clone();
            let service_detector_clone = service_detector.clone();

            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(
                    stream,
                    client_addr,
                    port,
                    session_tx_clone,
                    service_detector_clone,
                )
                .await
                {
                    error!(
                        "[!] Error handling connection from {}: {:?}",
                        client_addr, e
                    );
                }
            });
        }
    }

    pub fn shutdown() -> Result<(), NetworkError> {
        Ok(())
    }

    async fn handle_connection(
        stream: TcpStream,
        client_addr: SocketAddr,
        port: u16,
        session_tx: Sender<SessionRequest>,
        service_detector: ServiceDetector,
    ) -> Result<(), NetworkError> {
        let service_name = match service_detector.detect_service(&stream, port).await {
            Ok(name) => name,
            Err(e) => {
                log::warn!(
                    "[!] Failed to detect service for connection from {}: {:?}",
                    client_addr,
                    e
                );
                return Err(e);
            }
        };

        log::info!(
            "[+] Detected sevice '{:?}' for connection from {}",
            service_name,
            client_addr
        );

        // Create session request
        let session_request = SessionRequest {
            stream,
            service_name,
            client_addr,
            timestamp: Utc::now(),
        };

        if (session_tx.send(session_request).await).is_err() {
            log::error!("[!] Failed to send session request - channel may be closed");
            return Err(NetworkError::ChannelFailed);
        }

        log::debug!("[+] Session request sent successfully for {}", client_addr);

        Ok(())
    }

    /*
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
    */
}

#[cfg(test)]
mod tests {

    use super::*;
    use tokio::net::TcpListener;
    use tokio::sync::mpsc;
    use tokio::time;

    #[test]
    fn test_bind_services_from_legit_file() {
        // Build channel
        let (tx, _) = mpsc::channel(100);

        // Create new NetworkListener
        let mut listener = NetworkListener::new(tx);

        // Binding a server shouldn't return an error, if returns Err, panic! and test fails
        listener.bind_services(&[ServiceConfig::default()]).unwrap();
    }

    #[tokio::test]
    async fn test_start_listening_success() {
        let (session_tx, _) = mpsc::channel::<SessionRequest>(100);

        let mut network_listener = NetworkListener::new(session_tx);

        let service_config = ServiceConfig {
            port: 0,
            ..Default::default()
        };
        let _result = network_listener.bind_services(&[service_config]);

        // Start listening in a separate task with timeout since it runs indefinitely
        let listening_task = tokio::spawn(async move { network_listener.start_listening().await });

        // Give it a moment to start binding
        time::sleep(time::Duration::from_millis(100)).await;

        // The task should be running (not completed due to infinite loop)
        assert!(!listening_task.is_finished());

        // Cancel the task to clean up
        listening_task.abort();
    }

    /*
    #[tokio::test]
    async fn test_start_listening_bind_error() {
        let (session_tx, _session_rx) = mpsc::channel::<SessionRequest>(100);
        let service_detector = ServiceDetector::new();
        let connection_filter = ConnectionFilter::new();

        let mut network_manager =
            NetworkManager::new(session_tx, service_detector, connection_filter);

        // Try to bind to a privileged port that should fail
        network_manager.add_listener(80).unwrap();

        let result = network_manager.start_listening().await;

        // Should fail with bind error (unless running as root)
        assert!(matches!(result, Err(NetworkError::BindError(_))));
    }


    #[tokio::test]
    async fn test_start_listening_multiple_ports() {
        let (session_tx, _session_rx) = mpsc::channel::<SessionRequest>(100);
        let service_detector = ServiceDetector::new();
        let connection_filter = ConnectionFilter::new();

        let mut network_manager =
            NetworkManager::new(session_tx, service_detector, connection_filter);

        // Add multiple listeners
        network_manager.add_listener(0).unwrap(); // Available port 1
        network_manager.add_listener(0).unwrap(); // Available port 2

        let listening_task = tokio::spawn(async move { network_manager.start_listening().await });

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(!listening_task.is_finished());

        listening_task.abort();
    }
    */

    #[tokio::test]
    async fn test_listen_on_port_accepts_connections() {
        let (session_tx, mut session_rx) = mpsc::channel::<SessionRequest>(100);
        let service_detector = ServiceDetector::new(&[ServiceConfig::default()]);
        let connection_filter = ConnectionFilter::default();

        // Create a test listener
        let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
        let server_addr = listener.local_addr().unwrap();
        let port = server_addr.port();

        // Start listening in background
        let listen_task = tokio::spawn(async move {
            NetworkListener::listen_on_port(
                listener,
                session_tx,
                service_detector,
                connection_filter,
                port,
            )
            .await;
        });

        // Give the listener time to start
        time::sleep(time::Duration::from_millis(10)).await;

        // Connect to the listener
        let _client_stream = TcpStream::connect(server_addr).await.unwrap();

        // Should receive a session request (with timeout to avoid hanging)
        let session_result =
            time::timeout(time::Duration::from_millis(500), session_rx.recv()).await;
        assert!(session_result.is_ok());

        listen_task.abort();
    }

    /*
    #[tokio::test]
    async fn test_listen_on_port_filters_connections() {
        let (session_tx, mut session_rx) = mpsc::channel::<SessionRequest>(100);
        let service_detector = ServiceDetector::new();
        let mut connection_filter = ConnectionFilter::new();

        // Configure filter to reject connections (assuming you have such a method)
        connection_filter.set_reject_all(true);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server_addr = listener.local_addr().unwrap();
        let port = server_addr.port();

        let listen_task = tokio::spawn(async move {
            NetworkManager::listen_on_port(
                listener,
                session_tx,
                service_detector,
                connection_filter,
                port,
            )
            .await;
        });

        tokio::time::sleep(Duration::from_millis(10)).await;

        // Connect to the listener
        let _client_stream = TcpStream::connect(server_addr).await.unwrap();

        // Should NOT receive a session request due to filtering
        let session_result = timeout(Duration::from_millis(200), session_rx.recv()).await;
        assert!(session_result.is_err()); // Timeout expected

        listen_task.abort();
    }

    #[tokio::test]
    async fn test_listen_on_port_multiple_connections() {
        let (session_tx, mut session_rx) = mpsc::channel::<SessionRequest>(100);
        let service_detector = ServiceDetector::new();
        let connection_filter = ConnectionFilter::new();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server_addr = listener.local_addr().unwrap();
        let port = server_addr.port();

        let listen_task = tokio::spawn(async move {
            NetworkManager::listen_on_port(
                listener,
                session_tx,
                service_detector,
                connection_filter,
                port,
            )
            .await;
        });

        tokio::time::sleep(Duration::from_millis(10)).await;

        // Make multiple connections
        let num_connections = 3;
        for _ in 0..num_connections {
            let _client_stream = TcpStream::connect(server_addr).await.unwrap();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Should receive multiple session requests
        let mut received_count = 0;
        while received_count < num_connections {
            let session_result = timeout(Duration::from_millis(500), session_rx.recv()).await;
            if session_result.is_ok() {
                received_count += 1;
            } else {
                break;
            }
        }

        assert_eq!(received_count, num_connections);
        listen_task.abort();
    }
    */

    #[tokio::test]
    async fn test_handle_connection_success() {
        let (session_tx, mut session_rx) = mpsc::channel::<SessionRequest>(100);
        let service_detector = ServiceDetector::new(&[ServiceConfig::default()]);

        // Create a mock TCP connection
        let listener = TcpListener::bind("127.0.0.1:8000").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Connect to create a stream pair
        let _client_stream = TcpStream::connect(addr).await.unwrap();
        let (server_stream, client_addr) = listener.accept().await.unwrap();

        let result = NetworkListener::handle_connection(
            server_stream,
            client_addr,
            8080,
            session_tx,
            service_detector,
        )
        .await;

        assert!(result.is_ok());

        // Verify session request was sent
        let session_request = time::timeout(time::Duration::from_millis(100), session_rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(session_request.client_addr, client_addr);
        assert!(session_request.timestamp <= Utc::now());
    }

    /*
    #[tokio::test]
    async fn test_handle_connection_service_detection_failure() {
        let (session_tx, mut session_rx) = mpsc::channel::<SessionRequest>(100);
        let mut service_detector = ServiceDetector::new();

        // Configure service detector to fail (assuming you have such a method)
        service_detector.set_always_fail(true);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let client_stream = TcpStream::connect(addr).await.unwrap();
        let (server_stream, client_addr) = listener.accept().await.unwrap();

        let result = NetworkManager::handle_connection(
            server_stream,
            client_addr,
            8080,
            session_tx,
            service_detector,
        )
        .await;

        // Should return error
        assert!(result.is_err());

        // No session request should be sent
        let session_result = timeout(Duration::from_millis(100), session_rx.recv()).await;
        assert!(session_result.is_err()); // Timeout expected
    }

    #[tokio::test]
    async fn test_handle_connection_channel_closed() {
        let (session_tx, session_rx) = mpsc::channel::<SessionRequest>(1);
        let service_detector = ServiceDetector::new();

        // Close the receiver to simulate channel failure
        drop(session_rx);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let client_stream = TcpStream::connect(addr).await.unwrap();
        let (server_stream, client_addr) = listener.accept().await.unwrap();

        let result = NetworkManager::handle_connection(
            server_stream,
            client_addr,
            8080,
            session_tx,
            service_detector,
        )
        .await;

        // Should return channel error
        assert!(matches!(result, Err(NetworkError::ChannelFailed)));
    }
    */
    /*
    #[tokio::test]
    async fn test_start_listening_channel_communication() {
        let (tx, mut rx) = mpsc::channel(100);

        let mut listener = NetworkListener::new(tx);
        listener.bind_services(&[ServiceConfig::default()]).unwrap();

        listener.start_listening().await.unwrap();

        let received = rx.recv().await.unwrap();

        assert_eq!(received.service_name, "test_service".to_string());
    }
    */
}

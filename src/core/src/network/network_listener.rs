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

pub struct NetworkListener {
    listeners: HashMap<u16, TcpSocket>,
    session_tx: Sender<SessionRequest>,
    service_detector: ServiceDetector,
    connection_filter: ConnectionFilter,
}

impl NetworkListener {
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
    /// Binds services given in the `ServiceConfig` structure
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

    // First formulates a SessionRequest and send it through the mpsc channel
    //
    // If Ok response from SessionManager, bind with Configuration::bind_address (same IP for every
    // socket)
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

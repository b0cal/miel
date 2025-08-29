use super::connection_filter::*;
use super::service_detector::*;
use super::session_request::*;
use crate::configuration::types::{IpFilter, PortFilter, ServiceConfig};
use crate::error_handling::types::NetworkError;
use log::error;

use std::collections::HashMap;
use tokio::net::{TcpSocket, TcpStream};
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
            connection_filter: ConnectionFilter::new(),
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
    pub async fn start_listening(&self) -> Result<(), NetworkError> {
        Ok(())
    }
    pub fn shutdown() -> Result<(), NetworkError> {
        Ok(())
    }
    fn handle_connection(stream: TcpStream, service: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use tokio::sync::mpsc;
    use tokio_test::io::{Builder, Mock};

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

    #[test]
    fn test_bind_services_from_legit_file() {
        // Build channel
        let (tx, mut rx) = mpsc::channel(100);

        // Create new NetworkListener
        let mut listener = NetworkListener::new(tx);

        // Binding a server shouldn't return an error, if returns Err, panic! and test fails
        listener.bind_services(&[ServiceConfig::default()]).unwrap();
    }
}

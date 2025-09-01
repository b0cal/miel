use super::types::ServicePattern;
use crate::configuration::types::ServiceConfig;
use crate::error_handling::types::NetworkError;
use log::{error, info};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

#[derive(Clone)]
pub struct ServiceDetector {
    pub service_patterns: HashMap<u16, ServicePattern>,
}

impl ServiceDetector {
    pub fn new(services: &[ServiceConfig]) -> Self {
        let mut service_patterns = HashMap::new();

        for service in services {
            let pattern = ServicePattern {
                service_name: service.name.clone(),
                port: service.port,
                protocol: service.protocol.clone(),
                header_patterns: service.header_patterns.clone(),
                banner_patterns: match &service.banner_response {
                    Some(banner) => vec![banner.clone()],
                    None => Vec::new(),
                },
            };

            service_patterns.insert(pattern.port, pattern);
        }

        Self { service_patterns }
    }

    pub async fn identify_service(&self, stream: &mut TcpStream) -> Result<String, NetworkError> {
        let local_addr: SocketAddr = stream.local_addr().map_err(|e| {
            error!("Failed to get local_addr: {}", e);
            NetworkError::ServiceDetectionFailed
        })?;
        let port = local_addr.port();

        if let Some(service) = self.detect_from_port(port) {
            return Ok(service);
        }

        info!(
            "No service detected from the port {}, checking with payload...",
            port
        );

        let mut buf = [0u8; 1024];
        let n = stream.read(&mut buf).await.map_err(|e| {
            error!("failed to read from stream: {}", e);
            NetworkError::ServiceDetectionFailed
        })?;

        if n > 0 {
            if let Some(service) = self.detect_from_payload(&buf[..n]) {
                return Ok(service);
            }
        }

        info!("No service detected from port {} or payload", port);
        Err(NetworkError::ServiceDetectionFailed)
    }

    fn detect_from_port(&self, port: u16) -> Option<String> {
        self.service_patterns
            .get(&port)
            .map(|pattern| pattern.service_name.clone())
    }

    fn detect_from_payload(&self, data: &[u8]) -> Option<String> {
        let data_str = std::str::from_utf8(data).ok()?;

        self.service_patterns
            .values()
            .find(|service| {
                service
                    .header_patterns
                    .iter()
                    .any(|pattern| data_str.contains(pattern))
                    || service
                        .banner_patterns
                        .iter()
                        .any(|pattern| data_str.contains(pattern))
            })
            .map(|service| service.service_name.clone())
    }
}

use super::service_pattern::ServicePattern;
use crate::configuration::types::ServiceConfig;
use crate::error_handling::types::NetworkError;
use std::collections::HashMap;
use tokio::net::TcpStream;

#[derive(Clone)]
pub struct ServiceDetector {
    pub service_patterns: HashMap<String, Vec<ServicePattern>>,
}

impl ServiceDetector {
    pub fn new(services: &[ServiceConfig]) -> Self {
        services.len();
        Self {
            service_patterns: HashMap::new(),
        }
    }

    pub async fn detect_service(
        &self,
        stream: &TcpStream,
        port: u16,
    ) -> Result<String, NetworkError> {
        Ok("Ok".to_string())
    }
}

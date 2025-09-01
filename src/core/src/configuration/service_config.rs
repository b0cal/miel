use crate::configuration::types::Protocol;

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub name: String,
    pub port: u16,
    pub protocol: Protocol,
    pub container_image: String,
    pub enabled: bool,
    pub header_patterns: Vec<String>,
    pub banner_response: Option<String>,
}

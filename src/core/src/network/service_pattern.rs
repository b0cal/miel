use crate::configuration::types::Protocol;
pub struct ServicePattern {
    service_name: String,
    port: u16,
    protocol: Protocol,
    header_patterns: Vec<String>,
    banner_patterns: Vec<String>,
}

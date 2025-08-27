use std::net::IpAddr;
#[derive(Debug, PartialEq, Clone, Default)]
pub struct IpFilter {
    pub allowed_ranges: Vec<IpRange>,
    pub blocked_ranges: Vec<IpRange>,
    pub whitelist_mode: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub struct IpRange {
    pub start: IpAddr,
    pub end: IpAddr,
}
#[derive(Debug, PartialEq, Clone, Default)]
pub struct PortFilter {
    pub allowed_ports: Vec<PortRange>,
    pub blocked_ports: Vec<PortRange>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Protocol {
    TCP,
    UDP,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ServiceConfig {
    pub name: String,
    pub port: u16,
    pub protocol: Protocol,
    pub container_image: String,
    pub enabled: bool,
    pub header_patterns: Vec<String>,
    pub banner_response: Option<String>,
}

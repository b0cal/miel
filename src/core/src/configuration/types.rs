use serde::Deserialize;
use std::net::IpAddr;

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(default)]
pub struct IpFilter {
    #[serde(default)]
    pub allowed_ranges: Vec<IpRange>,
    #[serde(default)]
    pub blocked_ranges: Vec<IpRange>,
    #[serde(default)]
    pub whitelist_mode: bool,
}

impl Default for IpFilter {
    fn default() -> Self {
        Self {
            allowed_ranges: vec![IpRange::default()],
            blocked_ranges: vec![],
            whitelist_mode: false,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(default)]
pub struct IpRange {
    pub start: IpAddr,
    pub end: IpAddr,
}

impl Default for IpRange {
    fn default() -> Self {
        Self {
            start: "0.0.0.0".parse().unwrap(),
            end: "255.255.255.255".parse().unwrap(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(default)]
pub struct PortFilter {
    pub allowed_ports: Vec<PortRange>,
    pub blocked_ports: Vec<PortRange>,
}

impl Default for PortFilter {
    fn default() -> Self {
        Self {
            allowed_ports: vec![PortRange::default()],
            blocked_ports: vec![],
        }
    }
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(default)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

impl Default for PortRange {
    fn default() -> Self {
        Self {
            start: 1,
            end: 65535,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
pub enum Protocol {
    TCP,
    UDP,
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(default)]
pub struct ServiceConfig {
    pub name: String,
    pub port: u16,
    pub protocol: Protocol,
    pub container_image: String,
    pub enabled: bool,
    pub header_patterns: Vec<String>,
    pub banner_response: Option<String>,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            name: "unkown".to_string(),
            port: 0,
            protocol: Protocol::TCP,
            container_image: "unknown".to_string(),
            enabled: false,
            header_patterns: vec![],
            banner_response: None,
        }
    }
}

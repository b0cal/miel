use serde::Deserialize;
use std::net::IpAddr;

/// Storage backend options for the application
#[derive(Debug, PartialEq, Clone, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    #[value(name = "filesystem")]
    FileSystem,
    #[value(name = "database")]
    Database,
}

impl Default for StorageBackend {
    fn default() -> Self {
        Self::Database
    }
}

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
pub struct ServiceConfig {
    pub name: String,
    pub port: u16,
    pub protocol: Protocol,
    pub container_image: String,
    pub enabled: bool,
    pub header_patterns: Vec<String>,
    pub banner_response: Option<String>,
    pub obfuscation: ObfuscationConfig,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Default)]
#[serde(default)]
pub struct ObfuscationConfig {
    pub enabled: bool,
    pub fake_hostname: Option<String>,
    pub fake_processes: Vec<FakeProcess>,
    pub fake_files: Vec<FakeFile>,
    pub fake_users: Vec<String>,
    pub fake_network_interfaces: Vec<String>,
    pub system_uptime_days: Option<u32>,
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
pub struct FakeProcess {
    pub name: String,
    pub pid: Option<u32>,
    pub cpu_percent: Option<f32>,
    pub memory_mb: Option<u32>,
    pub command: String,
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
pub struct FakeFile {
    pub path: String,
    pub content: Option<String>,
    pub size_bytes: Option<u64>,
    pub is_executable: bool,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            name: "test_service".to_string(),
            port: 8000,
            protocol: Protocol::TCP,
            container_image: "container_image".to_string(),
            enabled: true,
            header_patterns: vec![],
            banner_response: None,
            obfuscation: ObfuscationConfig::default(),
        }
    }
}

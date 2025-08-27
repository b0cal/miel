use std::error::Error;
use clap::Parser;
use super::types::*;
use std::path::PathBuf;


/// Represents the configuration for the application.
/// This struct is currently empty but can be extended to include
/// fields and methods for managing application settings.
#[derive(Parser, Debug, Clone)]
struct Configuration {
    #[arg(skip)]
    services: Vec<ServiceConfig>,
    #[arg(long)]
    bind_address: String,
    #[arg(long)]
    storage_path: PathBuf,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    web_ui_enabled: bool,
    #[arg(long)]
    web_ui_port: u16,
    #[arg(long)]
    max_sessions: usize,
    #[arg(long)]
    session_timeout_secs: u64,
    #[arg(skip)]
    ip_filter: IpFilter,
    #[arg(skip)]
    port_filter: PortFilter
}

impl Configuration {
    /// Creates a new instance of `Configuration`.
    ///
    /// # Returns
    /// A new `Configuration` instance.
    /*
    pub fn new() -> Configuration {
        Self
    }
    */
    pub fn from_args() -> Self {
        Configuration::parse()
    }

    fn from_args_under_test() -> Result<Configuration, clap::Error>  {

        Configuration::try_parse_from(&[
            "miel",
            "--bind-address", "Test",
            "--storage-path", "/tmp",
            "--web-ui-enabled",
            "--web-ui-port", "0",
            "--max-sessions", "0",
            "--session-timeout-secs", "0"
        ])
    }


}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    use std::path::Path;

    fn sim_configuration() -> Configuration {
        let service = ServiceConfig {
            name: String::from("Name"),
            port: 0,
            protocol: Protocol::TCP,
            container_image: String::from("Image"),
            enabled: true,
            header_patterns: vec![String::from("Header")],
            banner_response: Option::from(String::from("Banner")),
        };
        let bind_address =  String::from("Test");
        let storage_path: PathBuf = PathBuf::from("/tmp");


        let ip_range1 = IpRange {
            start: IpAddr::V4(Ipv4Addr::new(127,0,0,1)),
            end: IpAddr::V4(Ipv4Addr::new(255,255,255, 255))
        };

        let ip_range2 = IpRange {
            start: IpAddr::V4(Ipv4Addr::new(127,0,0,1)),
            end: IpAddr::V4(Ipv4Addr::new(255,255,255, 255))
        };

        let ip_filter = IpFilter { allowed_ranges: vec![ip_range1], blocked_ranges: vec![ip_range2], whitelist_mode: true};


        let port_range1 = PortRange{start: 1, end: 2};
        let port_range2 = PortRange{start: 1, end: 2};

        let port_filter = PortFilter {allowed_ports: vec![port_range1], blocked_ports: vec![port_range2]};

        Configuration {
            web_ui_enabled: true,
            web_ui_port: 0,
            max_sessions: 0,
            session_timeout_secs: 0,
            services: vec![service],
            bind_address,
            storage_path,
            ip_filter,
            port_filter
        }
    }

    #[test]
    fn test_from_args() {

        let expected = sim_configuration();

        let config = Configuration::from_args_under_test().unwrap_or_else(|e| panic!("{}", e));

        //assert_eq!(config.services, expected.services);
        assert_eq!(config.bind_address, expected.bind_address);
        assert_eq!(config.storage_path, expected.storage_path);
        assert_eq!(config.web_ui_enabled, expected.web_ui_enabled);
        assert_eq!(config.web_ui_port, expected.web_ui_port);
        assert_eq!(config.max_sessions, expected.max_sessions);
        assert_eq!(config.session_timeout_secs, expected.session_timeout_secs);
        //assert_eq!(config.ip_filter, expected.ip_filter);
        //assert_eq!(config.port_filter, expected.port_filter);

    }

}

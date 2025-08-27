use super::types::*;
use clap::Parser;
use std::error::Error;
use std::path::PathBuf;

/// Application configuration structure that defines all runtime parameters.
///
/// This structure holds the complete configuration for the application, including
/// network settings, storage configuration, web UI settings, session management and filtering
/// rules. It uses the `clap` and `toml` derive macro for respectively command-line and file
/// argument parsing
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
///
/// // Parse configuration from command line arguments
/// let config = Configuration::from_args();
/// println!("Binding to: {}", config.bind_address);
/// println!("Storage path: {:?}", config.storage_path);
///
/// ```
///
/// # Fields Overview
///
/// The configuration contains the following attributes:
/// - `services`: a list of `ServiceConfig` used further by the *Container Manager* to configure
/// the services
/// - `bind_address`: For server binding
/// /// - `storage_path`: Path locating where the data should be stored when it's a file (?) + tbd
/// - `web_ui_enabled`: If `true`, will start the web UI service
/// - `web_ui_port`: Port on which to expose the web UI service
/// - `max_sessions`: Limiting the number of concurrent sessions to avoid DDOS and overload in
/// general
/// - `session_timeout_secs`: Lifetime duration of a given container
/// - `ip_filter`: Allows to filter ip ranges either to blacklist or white list them
/// - `port_filter`: Allows to filter port ranges either to blacklist or white list them

#[derive(Parser, Debug, Clone)]
struct Configuration {
    /// List of service configuration
    ///
    /// This field contains the configuration for all the services needing to be exposed through
    /// containers
    /// It is not exposed as a command-line argument for the moment as it requires to specify how
    /// to parse the Vec<ServiceConfig> object
    ///
    /// # Note
    /// Couldn't it be a string containing every service name we want to activate, but
    /// configuration files are already in a pre-defined directory where we can search for them by
    /// name ?
    ///
    /// Currently uses `#[arg(skip)]` to exclude from command-line parsing
    #[arg(skip)]
    services: Vec<ServiceConfig>,

    /// Network address to bind the server to.
    ///
    /// Specifies the IP address the server should listen for incoming connections.
    ///
    /// # Command Line
    /// Use `--bind-address <ADDRESS>` to set this value from the CLI
    #[arg(long)]
    bind_address: String,

    /// File system path for data storage.
    ///
    /// Specifies the directory where the application will store persistent data, application logs, session
    /// logs, etc.
    /// The path should be absolute
    ///
    /// # Command Line
    /// Use `--storage-path <PATH>` to set this value from the CLI
    #[arg(long)]
    storage_path: PathBuf,

    /// Enable or disable the web user interface
    ///
    /// When enabled, the application will serve a web UI that provides a dashboard for monitoring
    /// and exporting the data collected by the services. The web UI will be available on the port
    /// specified by `web_ui_port`
    ///
    /// # Command Line
    /// Use `--web-ui-enabled` flag to enable the web UI. This is a boolean flag that doesn't take
    /// a value - its presence enables the feature
    #[arg(long, action = clap::ArgAction::SetTrue)]
    web_ui_enabled: bool,

    /// Port number for the web user interface.
    ///
    /// Specifies the TCP port on which the web UI will be served when  `web_ui_enabled` is set to
    /// true. Port number should not be reserved by IANA so mostly in the range of 1024 - 65535
    /// both included
    ///
    /// # Command Line
    /// Use `--web-ui-port <PORT>` to set this value from the CLI
    #[arg(long)]
    web_ui_port: u16,

    /// Maximum number of concurrent sessions allowed
    ///
    /// Defines the upper limit for simultaneous active sessions that the application can handle.
    /// When this limit is reached, new session requests will be rejected.
    ///
    /// # Command Line
    /// Use `--max-sessions <COUNT>` to set this value from the CLI
    #[arg(long)]
    max_sessions: usize,

    /// Session timeout duration in seconds
    ///
    /// Specifies how long a session can remain inactive before it is automatically terminated.
    ///
    /// Setting this to '0' means sessions will never automatically be terminated
    ///
    /// # Command Line
    /// Use `--session-timeout-secs <SECONDS>` to set this value from the CLI
    #[arg(long)]
    session_timeout_secs: u64,

    /// IP address filtering configuration
    ///
    /// Contains allowed and blocked ranges of IP adresses, in addition to policy setting white
    /// list or blacklist mode
    ///
    /// # Note
    /// Uses `#[arg(skip)]` to exclude from command line parsing for the same reasons as `services`
    #[arg(skip)]
    ip_filter: IpFilter,

    /// Port filtering configuration
    ///
    /// Contains allowed and blocked ranges of ports
    ///
    /// # Note
    /// Uses `#[arg(skip)]` to exclude from command line parsing for the same reasons as `services`
    #[arg(skip)]
    port_filter: PortFilter,
}

impl Configuration {
    /*
    pub fn new() -> Configuration {
        Self
    }
    */

    /// Creates a new instance of `Configuration` by parsing either a configuration file or from
    /// the command line.
    ///
    /// This method uses the `clap` and `toml` parsers to respectively read the command-line
    /// arguments and a configuration file and constructs a `Configuration` instance.
    ///
    /// It automatically handles argument validation and error reporting for invalid arguments
    ///
    /// # Panics
    /// Panics if the command-line arguments cannot be parsed. This typically happens when required
    /// arguments are missing or invalid values are provided. The panic includes helpful error
    /// message for the user
    ///
    /// # Returns
    /// A new `Configuration` instance.
    pub fn from_args() -> Self {
        Configuration::parse()
    }

    fn from_args_under_test() -> Result<Configuration, clap::Error> {
        Configuration::try_parse_from(&[
            "miel",
            "--bind-address",
            "Test",
            "--storage-path",
            "/tmp",
            "--web-ui-enabled",
            "--web-ui-port",
            "0",
            "--max-sessions",
            "0",
            "--session-timeout-secs",
            "0",
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
        let bind_address = String::from("Test");
        let storage_path: PathBuf = PathBuf::from("/tmp");

        let ip_range1 = IpRange {
            start: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            end: IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)),
        };

        let ip_range2 = IpRange {
            start: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            end: IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)),
        };

        let ip_filter = IpFilter {
            allowed_ranges: vec![ip_range1],
            blocked_ranges: vec![ip_range2],
            whitelist_mode: true,
        };

        let port_range1 = PortRange { start: 1, end: 2 };
        let port_range2 = PortRange { start: 1, end: 2 };

        let port_filter = PortFilter {
            allowed_ports: vec![port_range1],
            blocked_ports: vec![port_range2],
        };

        Configuration {
            web_ui_enabled: true,
            web_ui_port: 0,
            max_sessions: 0,
            session_timeout_secs: 0,
            services: vec![service],
            bind_address,
            storage_path,
            ip_filter,
            port_filter,
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

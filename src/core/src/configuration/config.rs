use log::info;
use crate::error_handling::types::*;

/// Represents the configuration for the application.
/// This struct is currently empty but can be extended to include
/// fields and methods for managing application settings.
pub struct Config {
    foo: u16,
}

impl Config {
    /// Creates a new instance of `Configuration`.
    ///
    /// # Returns
    /// A new `Configuration` instance.
    pub fn from_file() -> Result<Config, ConfigError> {
       info!("Configuration file loaded from the TOML");
        Ok(Config { foo: 5 })
    }

    pub fn from_args() -> Result<Config, ConfigError> {
        info!("Configuration loaded from the command-line arguments");
        Ok(Config {foo: 5})
    }

}


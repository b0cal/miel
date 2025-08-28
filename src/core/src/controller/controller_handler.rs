use log::{debug, error, trace, warn, info};
use crate::configuration::config::Config;
use crate::error_handling::types::*;
use std::path::PathBuf;
use std::env;

/// Application structure that defines all runtines parameters
///
///
/// # Fields Overview
///
/// The configuration contains the following attributes:
/// - `config`: to understand the configuration to work with
pub struct Controller {
    /// A Config instance
    ///
    /// This field contains all information related to a configuration
    pub config: Config,
}


impl Controller {
    /// Allow to create an instance of a controller
    ///
    /// This function creates the controller configuration based on command line arguments
    /// or from a configuration file
    ///
    ///
    /// # Errors
    ///
    /// `ControllerError::Config` is returned if the configuration cannot be
    /// created from the file or the arguments
    ///
    /// # Returns
    ///
    /// A [`Result`] containing the initialized [`Controller`] on success but on failure
    /// a `ControllerError` is returned
    pub fn new() -> Result<Self, ControllerError> {
        info!("[+] INFO: called `new()` function in Controller");

        // If more than one arg is present that means flags were added to the startup command
        if env::args().len() > 1 {
            match Config::from_args() {
                Ok(config) =>  Ok(Self { config }),
                Err(err) => {
                   error!("[!]ERROR: {:?}", err);
                    Err(ControllerError::Config(err))
                }
            }
        } else { // Loading config from the file
            match Config::from_file() {
                Ok(config) => Ok(Self { config }),
                Err(err) => {
                    error!("[!] ERROR: {:?}", err);
                    Err(ControllerError::Config(err))
                }
            }
        }
    }

    pub fn run() {
        info!("[+] INFO: called `run()` function in Controller");
    }

    pub fn shutdown()  {
        info!("[+] INFO: called `shutdown()` function in Controller");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_new_with_args() {

    }

}
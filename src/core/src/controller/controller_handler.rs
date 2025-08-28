use log::{debug, error, trace, warn, info};
use crate::configuration::config::Config;
use crate::error_handling::types::*;
use std::path::PathBuf;
use std::env;
pub struct Controller {
    // Fields for the Controller struct
    pub config: Config,
}

// Constructor syntax
impl Controller {
    // Returns nothing for the moment
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
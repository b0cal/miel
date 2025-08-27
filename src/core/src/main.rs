use controller_handler::*;
use error_handling::*;
use miel::{types::ConfigError, *};

use log::{debug, error, info, trace, warn};

fn main() {
    let _controller = Controller::new();

    // Example how to log
    // https://docs.rs/env_logger/latest/env_logger/
    env_logger::init();

    //let e = ConfigError::InvalidFormat;
    // info!("informing {:?}", e); -> will only work if the enum implement Debug (add #[derive(Debug) to the enum]
    debug!("debugging");
    warn!("warning");
    error!("error");
    trace!("trace");

    // run with RUST_LOG=[target]=[level] cargo run, ex: RUST_LOG=miel cargo run -> displays every type of log
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

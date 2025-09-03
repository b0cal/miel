use clap::Parser;
use log::{error, info};
use miel::configuration::config::Config;
use miel::controller::controller_handler::Controller;
use std::path::Path;

#[derive(Parser)]
#[command(name = "miel")]
#[command(version = "0.0.1")]
#[command(about = "A comprehensive Chameleon Research Honeypot")]
struct Args {
    config_file: String,
}

#[tokio::main]
async fn main() {
    // Example how to log
    // https://docs.rs/env_logger/latest/env_logger/
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .format_target(false)
        .init();

    println!(
        "
██████╗  ██████╗  ██████╗ █████╗ ██╗         ██╗███╗   ███╗██╗███████╗██╗     
██╔══██╗██╔═████╗██╔════╝██╔══██╗██║        ██╔╝████╗ ████║██║██╔════╝██║     
██████╔╝██║██╔██║██║     ███████║██║       ██╔╝ ██╔████╔██║██║█████╗  ██║     
██╔══██╗████╔╝██║██║     ██╔══██║██║      ██╔╝  ██║╚██╔╝██║██║██╔══╝  ██║     
██████╔╝╚██████╔╝╚██████╗██║  ██║███████╗██╔╝   ██║ ╚═╝ ██║██║███████╗███████╗
╚═════╝  ╚═════╝  ╚═════╝╚═╝  ╚═╝╚══════╝╚═╝    ╚═╝     ╚═╝╚═╝╚══════╝╚══════╝
==============================================================================
              A comprehensive Chameleon Research Honeypot v0.0.1              
==============================================================================
"
    );

    info!("Importing configuration");

    // Get command-line arguments
    let args = Args::parse();

    if args.config_file.is_empty() {
        error!("No configuration file found");
        std::process::exit(1);
    }

    let config = Config::from_file(Path::new(args.config_file.as_str())).map_err(|e| {
        error!("Unable to import configuration from file: {:?}", e);
        std::process::exit(1);
    });

    info!("Configuration imported successfully");

    let mut controller = Controller::new(config.unwrap())
        .map_err(|e| {
            error!(
                "Unable to create a controller instance: {:?}, exiting...",
                e
            );
            std::process::exit(1);
        })
        .unwrap();

    let result = tokio::spawn(async move {
        info!("Spawning the controller");
        controller
            .run()
            .await
            .map_err(|e| {
                error!(
                    "Error occured in the controller process: {:?}, exiting...",
                    e
                )
            })
            .unwrap();
    });

    let _ = result.await.map_err(|e| {
        error!("Error joining at the end of execution: {:?}", e);
        std::process::exit(1);
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

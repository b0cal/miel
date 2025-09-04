use clap::Parser;
use log::{error, info};
use miel::configuration::config::Config;
use miel::controller::controller_handler::Controller;
use std::path::Path;
use tokio::signal;

#[derive(Parser)]
#[command(name = "miel")]
#[command(version = "0.0.1")]
#[command(about = "A comprehensive Chameleon Research Honeypot")]
struct Args {
    config_file: String,
}

#[tokio::main]
async fn main() {
    // Configure logging with specific levels the different modules
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .filter_module("sea_orm", log::LevelFilter::Warn)  // Reduce ORM logging
        .filter_module("sqlx", log::LevelFilter::Warn)     // Reduce SQLx logging
        .filter_module("sea_orm::query", log::LevelFilter::Error) // Suppress query logs
        .filter_module("sqlx::query", log::LevelFilter::Error)    // Suppress SQLx query logs
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

    info!("== Configuration import ==");

    // Get command-line arguments
    let args = Args::parse();

    if args.config_file.is_empty() {
        error!("No configuration file found, exiting...");
        std::process::exit(1);
    }

    let config = Config::from_file(Path::new(args.config_file.as_str())).map_err(|e| {
        error!("Unable to import configuration from file: {:?}, exiting", e);
        std::process::exit(1);
    });

    info!("Configuration imported successfully");

    info!("== Controller configuration ==");
    let mut controller = Controller::new(config.unwrap())
        .await
        .map_err(|e| {
            error!(
                "Unable to create a controller instance: {:?}, exiting...",
                e
            );
            std::process::exit(1);
        })
        .unwrap();

    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);

    let controller_handle = tokio::spawn(async move {
        if let Err(e) = controller.run(shutdown_rx).await {
            error!("Error occurred in the controller process: {:?}", e);
        }
    });

    info!("Controller operational!");

    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Received Ctrl+C signal, initiating graceful shutdown...");
        }
        Err(e) => {
            error!("Unable to listen for shutdown signal: {}", e);
        }
    }

    info!("Shutdown signal received, signaling controller to stop...");

    if let Err(e) = shutdown_tx.send(()) {
        error!("Failed to send shutdown signal: {:?}", e);
    }

    match tokio::time::timeout(tokio::time::Duration::from_secs(10), controller_handle).await {
        Ok(Ok(())) => {
            info!("Controller stopped and shut down gracefully");
        }
        Ok(Err(e)) => {
            error!("Controller task panicked: {:?}", e);
        }
        Err(_) => {
            error!("Controller shutdown timed out, forcing termination...");
        }
    }

    info!("Application shutdown complete");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

use clap::Parser;
use log::{error, info, warn};
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
    // Configure logging with specific levels for different modules
    // Respect RUST_LOG environment variable for overall level
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info) // Default level
        .filter_module("sea_orm", log::LevelFilter::Warn) // Reduce ORM logging
        .filter_module("sqlx", log::LevelFilter::Warn) // Reduce SQLx logging
        .filter_module("sea_orm::query", log::LevelFilter::Error) // Suppress query logs
        .filter_module("sqlx::query", log::LevelFilter::Error) // Suppress SQLx query logs
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
                  A comprehensive Chameleon Research Honeypot v{}
    ==============================================================================
    ",
        env!("CARGO_PKG_VERSION")
    );

    info!("Miel honeypot starting up");

    // Get command-line arguments
    let args = Args::parse();

    if args.config_file.is_empty() {
        error!("No configuration file specified");
        std::process::exit(1);
    }

    let config = Config::from_file(Path::new(args.config_file.as_str())).map_err(|e| {
        error!(
            "Failed to load configuration from {}: {:?}",
            args.config_file, e
        );
        std::process::exit(1);
    });

    info!("Configuration loaded from {}", args.config_file);

    let mut controller = Controller::new(config.unwrap())
        .await
        .map_err(|e| {
            error!("Failed to initialize controller: {:?}", e);
            std::process::exit(1);
        })
        .unwrap();

    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);

    let controller_handle = tokio::spawn(async move {
        if let Err(e) = controller.run(shutdown_rx).await {
            error!("Controller error: {:?}", e);
        }
    });

    info!("Miel honeypot is now operational");

    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Shutdown signal received, stopping honeypot...");
        }
        Err(e) => {
            error!("Failed to listen for shutdown signal: {}", e);
        }
    }

    if let Err(e) = shutdown_tx.send(()) {
        error!("Failed to send shutdown signal: {:?}", e);
    }

    match tokio::time::timeout(tokio::time::Duration::from_secs(10), controller_handle).await {
        Ok(Ok(())) => {
            info!("Miel honeypot shutdown completed");
        }
        Ok(Err(e)) => {
            error!("Controller task failed during shutdown: {:?}", e);
        }
        Err(_) => {
            warn!("Controller shutdown timed out after 10 seconds");
        }
    }
}

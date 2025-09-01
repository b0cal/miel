use log::{debug, error, info, warn};
use miel::configuration::{Protocol, ServiceConfig};
use miel::container_management::ContainerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger with environment variable support
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("Container Manager Demo");

    // Create a new container manager
    let mut container_manager = match ContainerManager::new() {
        Ok(mgr) => {
            info!("Container manager initialized successfully");
            mgr
        }
        Err(e) => {
            error!(
                "Failed to initialize ContainerManager (is systemd-nspawn installed and available?): {}",
                e
            );
            return Err(e.into());
        }
    };

    // Define a few service configurations (internal container ports are fixed)
    let ssh_service = ServiceConfig {
        name: "ssh".to_string(),
        port: 22,
        protocol: Protocol::TCP,
        container_image: "minimal-ssh".to_string(),
        enabled: true,
        header_patterns: vec!["SSH-2.0".to_string()],
        banner_response: Some("SSH-2.0-OpenSSH_8.0".to_string()),
    };

    let http_service = ServiceConfig {
        name: "http".to_string(),
        port: 80,
        protocol: Protocol::TCP,
        container_image: "minimal-http".to_string(),
        enabled: true,
        header_patterns: vec!["GET".to_string(), "POST".to_string()],
        banner_response: Some("HTTP/1.1 200 OK\r\nServer: nginx/1.18.0".to_string()),
    };

    info!("Service configurations created");
    debug!(
        "SSH service config: {} on port {}",
        ssh_service.name, ssh_service.port
    );
    debug!(
        "HTTP service config: {} on port {}",
        http_service.name, http_service.port
    );

    // Create containers for the services
    info!("Creating SSH container...");
    let _ssh_container = match container_manager.create_container(&ssh_service).await {
        Ok(container) => {
            info!(
                "SSH container created: {} (host:{} -> container:{})",
                container.id, container.host_port, container.port
            );
            container
        }
        Err(e) => {
            error!("Failed to create SSH container: {}", e);
            return Err(e.into());
        }
    };

    info!("Creating HTTP container...");
    let _http_container = match container_manager.create_container(&http_service).await {
        Ok(container) => {
            info!(
                "HTTP container created: {} (host:{} -> container:{})",
                container.id, container.host_port, container.port
            );
            container
        }
        Err(e) => {
            error!("Failed to create HTTP container: {}", e);
            return Err(e.into());
        }
    };

    // Display container statistics
    let stats = container_manager.get_container_stats();
    info!("Container Statistics");
    info!("Active containers: {}", stats.active_count);
    info!("Total created: {}", stats.total_created);
    info!("Failed containers: {}", stats.failed_count);

    if stats.total_created > 0 {
        let success_rate = ((stats.total_created - stats.failed_count) as f64
            / stats.total_created as f64)
            * 100.0;
        info!("Success rate: {:.1}%", success_rate);
    }

    // List active containers
    let active_containers = container_manager.list_active_containers();
    info!("Active Containers ({})", active_containers.len());
    for container_id in &active_containers {
        if let Some(container) = container_manager.get_container(container_id) {
            info!(
                "Container {} [{}] host:{} -> container:{} (Created: {})",
                container.id,
                container.service_name,
                container.host_port,
                container.port,
                container.created_at.format("%Y-%m-%d %H:%M:%S")
            );
        } else {
            warn!(
                "Container {} listed as active but not found in registry",
                container_id
            );
        }
    }

    info!("Containers are running. Connect to them on 127.0.0.1:<host_port> shown above.");
    info!("Press Ctrl+C to stop and clean up.");

    // Wait for Ctrl+C to terminate
    match tokio::signal::ctrl_c().await {
        Ok(()) => {
            info!("Ctrl+C received. Initiating graceful shutdown...");
        }
        Err(e) => {
            error!("Failed to listen for Ctrl+C signal: {}", e);
            warn!("Proceeding with cleanup anyway...");
        }
    }

    // Clean up containers gracefully
    info!("Cleaning up containers...");
    if let Err(e) = container_manager.cleanup_all_containers().await {
        error!("Cleanup error: {}", e);
        warn!("Some containers may not have been cleaned up properly");
    } else {
        info!("All containers cleaned up successfully");
    }

    let final_stats = container_manager.get_container_stats();
    info!(
        "Cleanup completed. Active containers: {} (total created: {})",
        final_stats.active_count, final_stats.total_created
    );

    if final_stats.active_count > 0 {
        warn!(
            "Warning: {} containers still active after cleanup",
            final_stats.active_count
        );
    }

    if final_stats.failed_count > 0 {
        warn!(
            "Note: {} operations failed during this demo session",
            final_stats.failed_count
        );
    }

    info!("Demo terminated");
    Ok(())
}

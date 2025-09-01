use miel::configuration::{Protocol, ServiceConfig};
use miel::container_management::ContainerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Container Manager Demo ===");

    // Create a new container manager
    let mut container_manager = match ContainerManager::new() {
        Ok(mgr) => mgr,
        Err(e) => {
            eprintln!(
                "Failed to initialize ContainerManager (is systemd-nspawn installed and available?): {}",
                e
            );
            return Err(e.into());
        }
    };
    println!("✓ Container manager initialized");

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

    println!("✓ Service configurations created");

    // Create containers for the services
    println!("Creating SSH honeypot container...");
    let ssh_container = container_manager.create_container(&ssh_service).await?;
    println!(
        "✓ SSH container created: {} (host:{} -> container:{})",
        ssh_container.id, ssh_container.host_port, ssh_container.port
    );

    println!("Creating HTTP honeypot container...");
    let http_container = container_manager.create_container(&http_service).await?;
    println!(
        "✓ HTTP container created: {} (host:{} -> container:{})",
        http_container.id, http_container.host_port, http_container.port
    );

    // Display container statistics
    let stats = container_manager.get_container_stats();
    println!("\n=== Container Statistics ===");
    println!("Active containers: {}", stats.active_count);
    println!("Total created: {}", stats.total_created);
    println!("Failed containers: {}", stats.failed_count);

    // List active containers
    let active_containers = container_manager.list_active_containers();
    println!("\n=== Active Containers ===");
    for container_id in &active_containers {
        if let Some(container) = container_manager.get_container(container_id) {
            println!(
                "- {} [{}] host:{} -> container:{} (Created: {})",
                container.id,
                container.service_name,
                container.host_port,
                container.port,
                container.created_at.format("%Y-%m-%d %H:%M:%S")
            );
        }
    }

    println!(
        "\nContainers are running. Connect to them on 127.0.0.1:<host_port> shown above.\nPress Ctrl+C to stop and clean up."
    );

    // Wait for Ctrl+C to terminate
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl+C");
    println!("\nCtrl+C received. Cleaning up containers...");

    // Clean up containers gracefully
    if let Err(e) = container_manager.cleanup_all_containers().await {
        eprintln!("Cleanup error: {}", e);
    }

    let final_stats = container_manager.get_container_stats();
    println!(
        "✓ Cleanup completed. Active containers: {} (total created: {})",
        final_stats.active_count, final_stats.total_created
    );

    println!("\n=== Demo terminated ===");
    Ok(())
}

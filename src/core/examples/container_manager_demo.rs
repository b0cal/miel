use log::{debug, error, info};
use miel::configuration::{Protocol, ServiceConfig};
use miel::container_management::ContainerManager;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("Starting container manager demo");

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

    // Service configurations
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

    // Create containers
    info!("Creating SSH container...");
    let ssh_container = container_manager.create_container(&ssh_service).await?;
    info!(
        "SSH container: {} (host:{})",
        ssh_container.id, ssh_container.host_port
    );

    info!("Creating HTTP container...");
    let http_container = container_manager.create_container(&http_service).await?;
    info!(
        "HTTP container: {} (host:{})",
        http_container.id, http_container.host_port
    );

    // Create external server sockets that clients will connect to
    let ssh_listener = TcpListener::bind("127.0.0.1:2222").await?;
    let http_listener = TcpListener::bind("127.0.0.1:8080").await?;

    info!("Demo servers listening:");
    info!(
        "  SSH proxy: 127.0.0.1:2222 -> container:{}",
        ssh_container.host_port
    );
    info!(
        "  HTTP proxy: 127.0.0.1:8080 -> container:{}",
        http_container.host_port
    );
    info!("Connect to these ports to interact with containers!");

    // Handle SSH connections
    let ssh_host_port = ssh_container.host_port;
    tokio::spawn(async move {
        while let Ok((client_socket, client_addr)) = ssh_listener.accept().await {
            info!("SSH client connected from: {}", client_addr);

            // Create connection to SSH container
            match TcpStream::connect(format!("127.0.0.1:{}", ssh_host_port)).await {
                Ok(container_socket) => {
                    info!("Linked SSH client {} to container", client_addr);
                    tokio::spawn(link_sockets(
                        client_socket,
                        container_socket,
                        "SSH".to_string(),
                    ));
                }
                Err(e) => error!("Failed to connect to SSH container: {}", e),
            }
        }
    });

    // Handle HTTP connections
    let http_host_port = http_container.host_port;
    tokio::spawn(async move {
        while let Ok((client_socket, client_addr)) = http_listener.accept().await {
            info!("HTTP client connected from: {}", client_addr);

            // Create connection to HTTP container
            match TcpStream::connect(format!("127.0.0.1:{}", http_host_port)).await {
                Ok(container_socket) => {
                    info!("Linked HTTP client {} to container", client_addr);
                    tokio::spawn(link_sockets(
                        client_socket,
                        container_socket,
                        "HTTP".to_string(),
                    ));
                }
                Err(e) => error!("Failed to connect to HTTP container: {}", e),
            }
        }
    });

    info!("Socket linking active. Try:");
    info!("  ssh -p 2222 miel@127.0.0.1");
    info!("  curl http://127.0.0.1:8080");
    info!("Press Ctrl+C to stop.");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");

    // Cleanup
    container_manager.cleanup_all_containers().await?;
    info!("Demo ended");
    Ok(())
}

/// Links two TCP sockets together for bidirectional communication
async fn link_sockets(client: TcpStream, container: TcpStream, service: String) {
    let (mut client_read, mut client_write) = client.into_split();
    let (mut container_read, mut container_write) = container.into_split();

    let service_client = service.clone();
    let service_container = service.clone();

    // Client to container forwarding
    let client_to_container = async move {
        let mut buffer = [0; 4096];
        loop {
            match client_read.read(&mut buffer).await {
                Ok(0) => {
                    debug!("{} client disconnected", service_client);
                    break;
                }
                Ok(n) => {
                    debug!("{} forwarding {} bytes to container", service_client, n);
                    if container_write.write_all(&buffer[..n]).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    };

    // Container to client forwarding
    let container_to_client = async move {
        let mut buffer = [0; 4096];
        loop {
            match container_read.read(&mut buffer).await {
                Ok(0) => {
                    debug!("{} container disconnected", service_container);
                    break;
                }
                Ok(n) => {
                    debug!("{} forwarding {} bytes to client", service_container, n);
                    if client_write.write_all(&buffer[..n]).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    };

    // Run both directions concurrently
    tokio::select! {
        _ = client_to_container => {},
        _ = container_to_client => {},
    }

    info!("{} connection closed", service);
}

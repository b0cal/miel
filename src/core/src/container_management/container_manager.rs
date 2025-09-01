use chrono::Utc;
use std::collections::HashMap;
use std::fs::File;
use std::process::Stdio;
use tokio::process::Command;
use uuid::Uuid;

use crate::configuration::service_config::ServiceConfig;
use crate::container_management::types::{ContainerHandle, ContainerStats, Runtime};
use crate::error_handling::types::ContainerError;

/// Orchestrates container lifecycle and bookkeeping for honeypot services.
///
/// The manager abstracts over a container runtime (currently [`Runtime::SystemdNspawn`])
/// and maintains a registry of active containers along with simple counters.
///
/// Design notes:
/// - Containers are created under `/tmp/miel-containers/<id>` and run with
///   `systemd-nspawn --ephemeral` and `--private-network`.
/// - A random ephemeral host port is allocated and mapped to the container's
///   internal service port.
/// - This is a minimal, best-effort implementation not meant for production isolation.
pub struct ContainerManager {
    runtime: Runtime,
    active_containers: HashMap<String, ContainerHandle>,
    stats: ContainerStats,
}

impl ContainerManager {
    /// Creates a new `ContainerManager`.
    ///
    /// Returns an error if the configured runtime is not available on the host.
    pub fn new() -> Result<Self, ContainerError> {
        // Check if systemd-nspawn is available, otherwise fail
        if !Self::is_runtime_available() {
            return Err(ContainerError::RuntimeNotAvailable);
        }

        Ok(ContainerManager {
            runtime: Runtime::SystemdNspawn,
            active_containers: HashMap::new(),
            stats: ContainerStats {
                active_count: 0,
                total_created: 0,
                failed_count: 0,
            },
        })
    }

    /// Creates a new container for the given `service_config` and returns its handle.
    ///
    /// Side effects:
    /// - Allocates an ephemeral host port (127.0.0.1) and maps it to the service port.
    /// - Spawns a `systemd-nspawn` process with an ephemeral rootfs under `/tmp`.
    /// - Updates internal stats and registry.
    ///
    /// Errors if the container cannot be prepared or started.
    pub async fn create_container(
        &mut self,
        service_config: &ServiceConfig,
    ) -> Result<ContainerHandle, ContainerError> {
        let container_id = format!(
            "miel-{}-{}",
            service_config.name,
            Uuid::new_v4().to_string()
        );

        // Use the runtime to create the container
        let handle = match self.runtime {
            Runtime::SystemdNspawn => {
                self.create_nspawn_container(service_config, &container_id)
                    .await?
            }
        };

        // Update stats
        self.stats.total_created += 1;
        self.stats.active_count += 1;

        // Store the container handle
        self.active_containers
            .insert(container_id.clone(), handle.clone());

        Ok(handle)
    }

    /// Cleans up a specific container.
    ///
    /// Best-effort: attempts to kill the process, remove the registry entry,
    /// decrement counters, and delete the temporary directory.
    pub async fn cleanup_container(
        &mut self,
        mut handle: ContainerHandle,
    ) -> Result<(), ContainerError> {
        // Kill the process if it's still running
        // TODO: consider waiting for graceful shutdown first
        if let Some(mut process) = handle.process_handle.take() {
            if let Err(e) = process.kill().await {
                eprintln!(
                    "Warning: Failed to kill container process {}: {}",
                    handle.id, e
                );
            }
        }

        // Remove from active containers
        self.active_containers.remove(&handle.id);
        self.stats.active_count = self.stats.active_count.saturating_sub(1);

        // Clean up container directory
        let container_path = format!("/tmp/miel-containers/{}", handle.id);
        if let Err(e) = std::fs::remove_dir_all(&container_path) {
            eprintln!(
                "Warning: Failed to clean up container directory {}: {}",
                container_path, e
            );
        }

        Ok(())
    }

    /// Cleans up all tracked containers, continuing on errors and counting failures.
    pub async fn cleanup_all_containers(&mut self) -> Result<(), ContainerError> {
        let container_handles: Vec<ContainerHandle> =
            self.active_containers.values().cloned().collect();

        for handle in container_handles {
            if let Err(e) = self.cleanup_container(handle).await {
                eprintln!("Failed to cleanup container: {}", e);
                self.stats.failed_count += 1;
            }
        }

        self.active_containers.clear();
        self.stats.active_count = 0;
        Ok(())
    }

    /// Returns a snapshot of current counters. `active_count` is recomputed
    /// from the current registry to stay accurate.
    pub fn get_container_stats(&self) -> ContainerStats {
        // Update active count to reflect current state
        let mut stats = self.stats.clone();
        stats.active_count = self.active_containers.len();
        stats
    }

    /// Returns a reference to an active container by id, if present.
    pub fn get_container(&self, container_id: &str) -> Option<&ContainerHandle> {
        self.active_containers.get(container_id)
    }

    /// Lists the identifiers of all active containers.
    pub fn list_active_containers(&self) -> Vec<String> {
        self.active_containers.keys().cloned().collect()
    }

    /// Checks whether the selected container runtime is available on the system.
    fn is_runtime_available() -> bool {
        std::process::Command::new("systemd-nspawn")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Creates a container using `systemd-nspawn` and returns its handle.
    async fn create_nspawn_container(
        &self,
        service_config: &ServiceConfig,
        container_id: &str,
    ) -> Result<ContainerHandle, ContainerError> {
        // Create a basic container directory structure
        let container_path = format!("/tmp/miel-containers/{}", container_id);
        std::fs::create_dir_all(&container_path).map_err(|e| {
            ContainerError::CreationFailed(format!("Failed to create container directory: {}", e))
        })?;

        // Create a basic rootfs structure
        self.setup_container_rootfs(&container_path, service_config)
            .await?;

        // Prepare the systemd-nspawn command
        let mut cmd = Command::new("systemd-nspawn");
        cmd.arg("--directory")
            .arg(&container_path)
            .arg("--ephemeral")
            .arg("--private-network")
            .arg("--bind-ro=/etc/resolv.conf")
            .arg(format!("--machine={}", container_id))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Allocate an ephemeral host port and add service-specific arguments
        let host_port = self.allocate_ephemeral_port(&service_config.protocol)?;
        match service_config.protocol {
            crate::configuration::types::Protocol::TCP => {
                // Map ephemeral host port to container's fixed internal service port
                cmd.arg(format!("--port={}:{}", host_port, service_config.port));
            }
            crate::configuration::types::Protocol::UDP => {
                cmd.arg(format!(
                    "--port={}:{}{}",
                    host_port, service_config.port, "/udp"
                ));
            }
        }

        // Add the service command to run
        cmd.arg("--").arg("/bin/sh").arg("-c");
        let service_command = self.get_service_command(service_config);
        cmd.arg(service_command);

        // Start the container process
        let process = cmd.spawn().map_err(|e| {
            ContainerError::StartFailed(format!("Failed to spawn container: {}", e))
        })?;

        // Create a PTY for stdio capture (placeholder implementation)
        let pty_master = self.create_pty_master(container_id).ok();

        let handle = ContainerHandle {
            id: container_id.to_string(),
            service_name: service_config.name.clone(),
            port: service_config.port,
            host_port,
            created_at: Utc::now(),
            process_handle: Some(process),
            pty_master,
            tcp_socket: None, // Will be set when connection is established
        };

        Ok(handle)
    }

    /// Sets up a minimal container rootfs with a dummy service script.
    async fn setup_container_rootfs(
        &self,
        container_path: &str,
        service_config: &ServiceConfig,
    ) -> Result<(), ContainerError> {
        // Create basic directory structure
        let dirs = ["bin", "usr/bin", "etc", "tmp", "var", "proc", "sys"];
        for dir in &dirs {
            let full_path = format!("{}/{}", container_path, dir);
            std::fs::create_dir_all(&full_path).map_err(|e| {
                ContainerError::CreationFailed(format!("Failed to create dir {}: {}", dir, e))
            })?;
        }

        // Copy essential binaries (simplified - in real implementation would use proper base image)
        if let Err(e) = std::fs::copy("/bin/sh", format!("{}/bin/sh", container_path)) {
            eprintln!("Warning: Failed to copy /bin/sh: {}", e);
        }

        // Create a simple service script based on the configuration
        let service_script = format!(
            "#!/bin/sh\necho 'Starting {} service on port {}'\nwhile true; do\n    echo 'Service {} is running'\n    sleep 30\ndone\n",
            service_config.name, service_config.port, service_config.name
        );

        std::fs::write(
            format!("{}/usr/bin/service", container_path),
            service_script,
        )
        .map_err(|e| {
            ContainerError::CreationFailed(format!("Failed to create service script: {}", e))
        })?;

        // Make service script executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(format!("{}/usr/bin/service", container_path))
                .map_err(|e| {
                    ContainerError::CreationFailed(format!("Failed to get script metadata: {}", e))
                })?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(format!("{}/usr/bin/service", container_path), perms)
                .map_err(|e| {
                    ContainerError::CreationFailed(format!(
                        "Failed to set script permissions: {}",
                        e
                    ))
                })?;
        }

        Ok(())
    }

    /// Returns the command line to run for a given `service_config`.
    fn get_service_command(&self, service_config: &ServiceConfig) -> String {
        // Return the service command based on the service configuration
        // In a real implementation, this would be more sophisticated
        match service_config.name.as_str() {
            // FIXME: replace with actual services.
            "ssh" => format!("nc -l -p {} -k", service_config.port),
            "http" => format!("nc -l -p {} -k", service_config.port),
            _ => format!("/usr/bin/service"),
        }
    }

    /// Creates a PTY master used to capture stdio. Placeholder implementation.
    fn create_pty_master(&self, container_id: &str) -> Result<File, ContainerError> {
        // TODO: Implement actual PTY creation logic
        // For now, create a temporary file as placeholder
        let pty_path = format!("/tmp/miel-pty-{}", container_id);

        std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(&pty_path)
            .map_err(|e| ContainerError::CreationFailed(format!("Failed to create PTY: {}", e)))
    }

    /// Allocates an ephemeral host port on 127.0.0.1 for the given protocol.
    fn allocate_ephemeral_port(
        &self,
        protocol: &crate::configuration::types::Protocol,
    ) -> Result<u16, ContainerError> {
        match protocol {
            crate::configuration::types::Protocol::TCP => {
                let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
                    .map_err(|e| {
                        ContainerError::CreationFailed(format!(
                            "Failed to allocate ephemeral TCP port: {}",
                            e
                        ))
                    })?;
                let port = listener
                    .local_addr()
                    .map_err(|e| {
                        ContainerError::CreationFailed(format!(
                            "Failed to read local addr for ephemeral TCP port: {}",
                            e
                        ))
                    })?
                    .port();
                // Close the socket to free the port for nspawn to bind
                drop(listener);
                Ok(port)
            }
            crate::configuration::types::Protocol::UDP => {
                let socket = std::net::UdpSocket::bind((std::net::Ipv4Addr::LOCALHOST, 0))
                    .map_err(|e| {
                        ContainerError::CreationFailed(format!(
                            "Failed to allocate ephemeral UDP port: {}",
                            e
                        ))
                    })?;
                let port = socket
                    .local_addr()
                    .map_err(|e| {
                        ContainerError::CreationFailed(format!(
                            "Failed to read local addr for ephemeral UDP port: {}",
                            e
                        ))
                    })?
                    .port();
                // Close the socket to free the port for nspawn to bind
                drop(socket);
                Ok(port)
            }
        }
    }
}

impl Drop for ContainerManager {
    fn drop(&mut self) {
        // Attempt to cleanup all containers when the manager is dropped
        if !self.active_containers.is_empty() {
            eprintln!(
                "Warning: ContainerManager dropped with {} active containers",
                self.active_containers.len()
            );
        }
    }
}

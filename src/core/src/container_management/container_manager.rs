use chrono::Utc;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
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
        info!("Initializing ContainerManager");

        // Check if systemd-nspawn is available, otherwise fail
        if !Self::is_runtime_available() {
            error!("systemd-nspawn runtime is not available on this system");
            return Err(ContainerError::RuntimeNotAvailable);
        }

        // Require root privileges: unprivileged nspawn with a plain directory tree is not supported
        // on many systems and will implicitly enable private networking, breaking host-port binding.
        if !Self::is_running_as_root() {
            error!("Insufficient privileges: systemd-nspawn requires root access");
            return Err(ContainerError::StartFailed(
                "systemd-nspawn requires root privileges for this setup. Please run the program with sudo.".to_string(),
            ));
        }

        let manager = ContainerManager {
            runtime: Runtime::SystemdNspawn,
            active_containers: HashMap::new(),
            stats: ContainerStats {
                active_count: 0,
                total_created: 0,
                failed_count: 0,
            },
        };

        info!(
            "ContainerManager initialized successfully with runtime: {:?}",
            manager.runtime
        );
        Ok(manager)
    }

    /// Best-effort check for root privileges (effective UID == 0).
    fn is_running_as_root() -> bool {
        // Avoid extra deps; use a small shell to query id -u
        let is_root = if let Ok(output) = std::process::Command::new("id").arg("-u").output() {
            if output.status.success() {
                if let Ok(s) = String::from_utf8(output.stdout) {
                    s.trim() == "0"
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        debug!("Root privilege check result: {}", is_root);
        is_root
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

        info!(
            "Creating container {} for service {}",
            container_id, service_config.name
        );

        // Use the runtime to create the container
        let handle = match self.runtime {
            Runtime::SystemdNspawn => {
                debug!("Using SystemdNspawn runtime for container creation");
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

        info!(
            "Successfully created and registered container: {}",
            container_id
        );
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
        info!("Starting cleanup for container: {}", handle.id);

        // Kill the process if it's still running
        // TODO: consider waiting for graceful shutdown first
        if let Some(mut process) = handle.process_handle.take() {
            debug!("Terminating process for container: {}", handle.id);
            if let Err(e) = process.kill().await {
                warn!("Failed to kill container process {}: {}", handle.id, e);
            } else {
                debug!(
                    "Successfully terminated process for container: {}",
                    handle.id
                );
            }
        } else {
            debug!("No process handle found for container: {}", handle.id);
        }

        // Remove from active containers
        self.active_containers.remove(&handle.id);
        self.stats.active_count = self.stats.active_count.saturating_sub(1);

        // Clean up container directory
        let container_path = format!("/tmp/miel-containers/{}", handle.id);
        debug!("Cleaning up container directory: {}", container_path);
        if let Err(e) = std::fs::remove_dir_all(&container_path) {
            warn!(
                "Failed to clean up container directory {}: {}",
                container_path, e
            );
        } else {
            debug!(
                "Successfully cleaned up container directory: {}",
                container_path
            );
        }

        info!("Completed cleanup for container: {}", handle.id);
        Ok(())
    }

    /// Cleans up all tracked containers, continuing on errors and counting failures.
    pub async fn cleanup_all_containers(&mut self) -> Result<(), ContainerError> {
        let container_count = self.active_containers.len();
        info!("Starting cleanup of {} active containers", container_count);

        let container_handles: Vec<ContainerHandle> =
            self.active_containers.values().cloned().collect();

        for handle in container_handles {
            if let Err(e) = self.cleanup_container(handle).await {
                error!("Failed to cleanup container: {}", e);
                self.stats.failed_count += 1;
            }
        }

        self.active_containers.clear();
        self.stats.active_count = 0;
        info!(
            "Completed cleanup of all containers (failures: {})",
            self.stats.failed_count
        );
        Ok(())
    }

    /// Returns a snapshot of current counters. `active_count` is recomputed
    /// from the current registry to stay accurate.
    pub fn get_container_stats(&self) -> ContainerStats {
        // Update active count to reflect current state
        let mut stats = self.stats.clone();
        stats.active_count = self.active_containers.len();
        debug!(
            "Retrieved container stats: active={}, total={}, failed={}",
            stats.active_count, stats.total_created, stats.failed_count
        );
        stats
    }

    /// Returns a reference to an active container by id, if present.
    pub fn get_container(&self, container_id: &str) -> Option<&ContainerHandle> {
        let result = self.active_containers.get(container_id);
        debug!(
            "Container lookup for {}: {}",
            container_id,
            if result.is_some() {
                "found"
            } else {
                "not found"
            }
        );
        result
    }

    /// Lists the identifiers of all active containers.
    pub fn list_active_containers(&self) -> Vec<String> {
        let ids = self.active_containers.keys().cloned().collect::<Vec<_>>();
        debug!("Listed {} active containers", ids.len());
        ids
    }

    /// Checks whether the selected container runtime is available on the system.
    fn is_runtime_available() -> bool {
        let available = std::process::Command::new("systemd-nspawn")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);

        debug!("systemd-nspawn availability check: {}", available);
        available
    }

    /// Creates a container using `systemd-nspawn` and returns its handle.
    async fn create_nspawn_container(
        &self,
        service_config: &ServiceConfig,
        container_id: &str,
    ) -> Result<ContainerHandle, ContainerError> {
        debug!("Starting nspawn container creation for: {}", container_id);

        // Create a basic container directory structure
        let container_path = format!("/tmp/miel-containers/{}", container_id);
        debug!("Preparing container directory: {}", container_path);
        std::fs::create_dir_all(&container_path).map_err(|e| {
            error!(
                "Failed to create container directory {}: {}",
                container_path, e
            );
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
            .arg("--bind-ro=/etc/resolv.conf")
            // Bind essential host dirs so common binaries and their libs are available
            // inside the minimal rootfs. Only bind paths that exist on the host.
            ;

        let mut bound_paths = 0;
        for p in [
            "/bin",
            "/usr/bin",
            "/sbin",
            "/usr/sbin",
            "/usr/libexec",
            "/lib",
            "/lib64",
            "/usr/lib",
            "/usr/lib64",
        ]
        .iter()
        {
            if Path::new(p).exists() {
                cmd.arg(format!("--bind-ro={}", p));
                bound_paths += 1;
            }
        }
        debug!(
            "Bound {} system paths for container {}",
            bound_paths, container_id
        );

        cmd.arg(format!("--machine={}", container_id))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Allocate an ephemeral host port. We will listen directly on this port
        // from inside the container (no nspawn port mapping), so the container
        // must share the host network namespace.
        let host_port = self.allocate_ephemeral_port(&service_config.protocol)?;
        debug!(
            "Allocated ephemeral port {} for container {}",
            host_port, container_id
        );

        // Add the service command to run
        cmd.arg("--").arg("/bin/sh").arg("-c");
        let service_command = self.get_service_command(service_config, host_port);
        cmd.arg(&service_command);

        debug!(
            "Spawning nspawn process for container {} with command: {:?}",
            container_id, service_command
        );

        // Start the container process
        let mut process = cmd.spawn().map_err(|e| {
            error!("Failed to spawn container {}: {}", container_id, e);
            ContainerError::StartFailed(format!("Failed to spawn container: {}", e))
        })?;

        // Capture stderr to help diagnose issues (e.g., missing binaries/options)
        if let Some(stderr) = process.stderr.take() {
            let mut reader = BufReader::new(stderr).lines();
            let cid = container_id.to_string();
            tokio::spawn(async move {
                while let Ok(Some(line)) = reader.next_line().await {
                    debug!("[nspawn:{}][stderr] {}", cid, line);
                }
                debug!("stderr monitoring ended for container: {}", cid);
            });
        }

        // Create a PTY for stdio capture (placeholder implementation)
        let pty_master = self.create_pty_master(container_id).ok();
        // Capture stdout for additional context
        if let Some(stdout) = process.stdout.take() {
            let mut reader = BufReader::new(stdout).lines();
            let cid = container_id.to_string();
            tokio::spawn(async move {
                while let Ok(Some(line)) = reader.next_line().await {
                    debug!("[nspawn:{}][stdout] {}", cid, line);
                }
                debug!("stdout monitoring ended for container: {}", cid);
            });
        }

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

        debug!("Successfully created nspawn container: {}", container_id);
        Ok(handle)
    }

    /// Sets up a minimal container rootfs with a dummy service script.
    async fn setup_container_rootfs(
        &self,
        container_path: &str,
        service_config: &ServiceConfig,
    ) -> Result<(), ContainerError> {
        debug!("Setting up container rootfs at: {}", container_path);

        // Create basic directory structure
        let dirs = [
            "bin",
            "usr/bin",
            "sbin",
            "usr/sbin",
            "usr/libexec",
            "usr/libexec/openssh",
            "etc",
            "etc/ssh",
            "var",
            "var/run",
            "var/run/sshd",
            "tmp",
            "proc",
            "sys",
            "www",
            "root",
            "home",
            "home/miel",
            "usr/share",
            "usr/share/empty.sshd",
        ];

        debug!("Creating {} directories in container", dirs.len());
        for dir in &dirs {
            let full_path = format!("{}/{}", container_path, dir);
            std::fs::create_dir_all(&full_path).map_err(|e| {
                error!("Failed to create directory {}: {}", full_path, e);
                ContainerError::CreationFailed(format!("Failed to create dir {}: {}", dir, e))
            })?;
        }

        // Copy essential binaries (simplified - in real implementation would use proper base image)
        debug!("Copying essential binaries to container");
        if let Err(e) = std::fs::copy("/bin/sh", format!("{}/bin/sh", container_path)) {
            warn!("Failed to copy /bin/sh: {}", e);
        } else {
            debug!("Successfully copied /bin/sh");
        }

        // Provide minimal system files for sshd compatibility
        let files = [
            ("etc/passwd", "root:x:0:0:root:/root:/bin/sh\nsshd:x:74:74:sshd privilege separation user:/var/run/sshd:/bin/false\nmiel:x:1000:1000:miel User:/home/miel:/bin/sh\n"),
            ("etc/group", "root:x:0:\nsshd:x:74:\nmiel:x:1000:\n"),
            ("etc/shadow", "root:*:19000:0:99999:7:::\nsshd:*:19000:0:99999:7:::\nmiel:$6$JWGaRU6XKVQ3ONQJ$k/G2q0uMScsEKSjfMS6YteGEEGuOl2wdodXeU6QSQSsBXOC1wG0TPcmWvsa1elj7P4LCmKy9P1NcStmATA6h11:19000:0:99999:7:::\n"),
            ("www/index.html", "<html><body><h1>Miel demo</h1><p>It works.</p></body></html>\n"),
        ];

        debug!("Creating {} system files", files.len());
        for (path, content) in &files {
            let full_path = format!("{}/{}", container_path, path);
            if let Err(e) = std::fs::write(&full_path, content) {
                warn!("Failed to write {}: {}", path, e);
            } else {
                debug!("Successfully created system file: {}", path);
            }
        }

        // Create a simple service script based on the configuration
        debug!("Creating service files for: {}", service_config.name);
        let service_script = format!(
            "#!/bin/sh\necho 'Starting {} service on port {}'\nwhile true; do\n    echo 'Service {} is running'\n    sleep 30\ndone\n",
            service_config.name, service_config.port, service_config.name
        );

        std::fs::write(
            format!("{}/usr/bin/service", container_path),
            service_script,
        )
        .map_err(|e| {
            error!("Failed to create service script: {}", e);
            ContainerError::CreationFailed(format!("Failed to create service script: {}", e))
        })?;

        // Make service script executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            debug!("Setting executable permissions for service script");
            let mut perms = std::fs::metadata(format!("{}/usr/bin/service", container_path))
                .map_err(|e| {
                    error!("Failed to get script metadata: {}", e);
                    ContainerError::CreationFailed(format!("Failed to get script metadata: {}", e))
                })?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(format!("{}/usr/bin/service", container_path), perms)
                .map_err(|e| {
                    error!("Failed to set script permissions: {}", e);
                    ContainerError::CreationFailed(format!(
                        "Failed to set script permissions: {}",
                        e
                    ))
                })?;
        }

        debug!("Successfully set up container rootfs");
        Ok(())
    }

    /// Returns the command line to run for a given `service_config`.
    fn get_service_command(&self, service_config: &ServiceConfig, host_port: u16) -> String {
        // Return the service command based on the service configuration
        // In a real implementation, this would be more sophisticated
        let command = match service_config.name.as_str() {
            "ssh" => {
                let p = host_port;
                // Generate host keys if missing and run OpenSSH sshd in foreground
                // Enable password authentication for honeypot purposes
                format!(
                    "/usr/bin/ssh-keygen -A >/dev/null 2>&1 || /bin/ssh-keygen -A >/dev/null 2>&1; /usr/sbin/sshd -D -e -f /dev/null -p {p} -o ListenAddress=127.0.0.1 -o UsePAM=no -o PasswordAuthentication=yes -o PermitRootLogin=no -o PidFile=/var/run/sshd/sshd.pid"
                )
            }
            "http" => {
                let p = host_port;
                // Use Python's built-in HTTP server
                format!("/usr/bin/python3 -m http.server {p} --bind 127.0.0.1 --directory /www")
            }
            _ => format!("/bin/sh /usr/bin/service"),
        };

        debug!(
            "Generated service command for {}: {}",
            service_config.name, command
        );
        command
    }

    /// Creates a PTY master used to capture stdio. Placeholder implementation.
    fn create_pty_master(&self, container_id: &str) -> Result<File, ContainerError> {
        // TODO: Implement actual PTY creation logic
        // For now, create a temporary file as placeholder
        let pty_path = format!("/tmp/miel-pty-{}", container_id);
        debug!("Creating PTY master at: {}", pty_path);

        std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(&pty_path)
            .map_err(|e| {
                error!("Failed to create PTY {}: {}", pty_path, e);
                ContainerError::CreationFailed(format!("Failed to create PTY: {}", e))
            })
    }

    /// Allocates an ephemeral host port on 127.0.0.1 for the given protocol.
    fn allocate_ephemeral_port(
        &self,
        protocol: &crate::configuration::types::Protocol,
    ) -> Result<u16, ContainerError> {
        let port = match protocol {
            crate::configuration::types::Protocol::TCP => {
                let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
                    .map_err(|e| {
                        error!("Failed to allocate ephemeral TCP port: {}", e);
                        ContainerError::CreationFailed(format!(
                            "Failed to allocate ephemeral TCP port: {}",
                            e
                        ))
                    })?;
                let port = listener
                    .local_addr()
                    .map_err(|e| {
                        error!("Failed to read local addr for ephemeral TCP port: {}", e);
                        ContainerError::CreationFailed(format!(
                            "Failed to read local addr for ephemeral TCP port: {}",
                            e
                        ))
                    })?
                    .port();
                // Close the socket to free the port for nspawn to bind
                drop(listener);
                debug!("Allocated TCP port: {}", port);
                port
            }
            crate::configuration::types::Protocol::UDP => {
                let socket = std::net::UdpSocket::bind((std::net::Ipv4Addr::LOCALHOST, 0))
                    .map_err(|e| {
                        error!("Failed to allocate ephemeral UDP port: {}", e);
                        ContainerError::CreationFailed(format!(
                            "Failed to allocate ephemeral UDP port: {}",
                            e
                        ))
                    })?;
                let port = socket
                    .local_addr()
                    .map_err(|e| {
                        error!("Failed to read local addr for ephemeral UDP port: {}", e);
                        ContainerError::CreationFailed(format!(
                            "Failed to read local addr for ephemeral UDP port: {}",
                            e
                        ))
                    })?
                    .port();
                // Close the socket to free the port for nspawn to bind
                drop(socket);
                debug!("Allocated UDP port: {}", port);
                port
            }
        };

        debug!("Allocated ephemeral {:?} port: {}", protocol, port);
        Ok(port)
    }
}

impl Drop for ContainerManager {
    fn drop(&mut self) {
        // Attempt to cleanup all containers when the manager is dropped
        if !self.active_containers.is_empty() {
            warn!(
                "ContainerManager dropped with {} active containers - this may indicate a resource leak",
                self.active_containers.len()
            );

            // Log the IDs of remaining containers for debugging
            let remaining_ids: Vec<_> = self.active_containers.keys().collect();
            warn!("Remaining container IDs: {:?}", remaining_ids);
        } else {
            debug!("ContainerManager dropped cleanly with no active containers");
        }
    }
}

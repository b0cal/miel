use chrono::Utc;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;
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

    /// Best-effort check for root privileges (EUID == 0).
    fn is_running_as_root() -> bool {
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
        let container_id = format!("miel-{}-{}", service_config.name, Uuid::new_v4());

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

        // Store the handle
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

        // Prepare systemd-nspawn command
        let mut cmd = Command::new("systemd-nspawn");
        cmd.arg("--directory")
            .arg(&container_path)
            .arg("--ephemeral")
            .arg("--bind-ro=/etc/resolv.conf");

        // Create and bind the log directory so containers can write to it
        let log_dir = "/tmp/miel-logs";
        std::fs::create_dir_all(log_dir).map_err(|e| {
            error!("Failed to create log directory {}: {}", log_dir, e);
            ContainerError::CreationFailed(format!("Failed to create log directory: {}", e))
        })?;

        // Set permissions on the log directory to be writable by all users
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(log_dir)
                .map_err(|e| {
                    error!("Failed to get log directory metadata: {}", e);
                    ContainerError::CreationFailed(format!("Failed to get log directory metadata: {}", e))
                })?
                .permissions();
            perms.set_mode(0o777); // rwxrwxrwx - allow all users to write
            std::fs::set_permissions(log_dir, perms)
                .map_err(|e| {
                    error!("Failed to set log directory permissions: {}", e);
                    ContainerError::CreationFailed(format!("Failed to set log directory permissions: {}", e))
                })?;
        }

        // Bind the log directory so it's accessible from within the container
        cmd.arg(format!("--bind={}", log_dir));
        debug!("Bound log directory {} to container {}", log_dir, container_id);

        // Bind essential host dirs so common binaries and their libs are available
        // inside the minimal rootfs. Only bind paths that exist on the host.
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

        // Allocate an ephemeral host port for the service
        let host_port = self.allocate_ephemeral_port(&service_config.protocol)?;
        debug!(
            "Allocated ephemeral port {} for container {}",
            host_port, container_id
        );

        // Add the service command to run
        cmd.arg("--").arg("/bin/sh").arg("-c");
        let service_command = self.get_service_command(service_config, host_port, container_id);
        cmd.arg(&service_command);

        debug!(
            "Spawning nspawn process for container {} with command: {:?}",
            container_id, service_command
        );

        // Start the process
        let mut process = cmd.spawn().map_err(|e| {
            error!("Failed to spawn container {}: {}", container_id, e);
            ContainerError::StartFailed(format!("Failed to spawn container: {}", e))
        })?;

        // Capture stderr
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

        // Create a PTY for stdio capture - now creates unified activity log
        let pty_master = self.create_pty_master(container_id).ok();

        // Capture stdout and redirect to unified log file
        if let Some(stdout) = process.stdout.take() {
            let log_path = format!("/tmp/miel-logs/container-{}-activity.log", container_id);
            let mut reader = BufReader::new(stdout).lines();
            let cid = container_id.to_string();
            tokio::spawn(async move {
                while let Ok(Some(line)) = reader.next_line().await {
                    debug!("[nspawn:{}][stdout] {}", cid, line);
                    // Also write to the unified log file
                    if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(&log_path) {
                        use std::io::Write;
                        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
                        let _ = writeln!(file, "[{}] [CONTAINER] {}", timestamp, line);
                        let _ = file.flush();
                    }
                }
                debug!("stdout monitoring ended for container: {}", cid);
            });
        }

        // Wait for the service to start up and establish a TCP connection
        info!(
            "Waiting for service to start and establishing TCP connection to container {}",
            container_id
        );
        let tcp_socket = self
            .establish_container_connection(host_port, container_id)
            .await?;

        let handle = ContainerHandle {
            id: container_id.to_string(),
            service_name: service_config.name.clone(),
            port: service_config.port,
            host_port,
            created_at: Utc::now(),
            process_handle: Some(process),
            pty_master,
            tcp_socket: Some(tcp_socket),
        };

        debug!(
            "Successfully created nspawn container with TCP connection: {}",
            container_id
        );
        Ok(handle)
    }

    /// Establishes a TCP connection to the container service with retry logic.
    ///
    /// This method waits for the service inside the container to start up and
    /// then establishes a TCP connection that can be used for traffic forwarding.
    async fn establish_container_connection(
        &self,
        host_port: u16,
        container_id: &str,
    ) -> Result<TcpStream, ContainerError> {
        let max_retries = 30;
        let mut retries = 0;
        let target_addr = format!("127.0.0.1:{}", host_port);

        debug!(
            "Attempting to connect to container service at {}",
            target_addr
        );

        while retries < max_retries {
            match TcpStream::connect(&target_addr).await {
                Ok(socket) => {
                    info!(
                        "Successfully established TCP connection to container {} on {}",
                        container_id, target_addr
                    );
                    return Ok(socket);
                }
                Err(e) => {
                    retries += 1;
                    let wait_time = std::cmp::min(500 + (retries * 200), 3000); // Progressive backoff, cap at 3s
                    debug!(
                        "Connection attempt {}/{} failed for container {}: {} - retrying in {}ms",
                        retries, max_retries, container_id, e, wait_time
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(wait_time)).await;
                }
            }
        }

        Err(ContainerError::ConnectionFailed(format!(
            "Failed to establish TCP connection to container {} at {} after {} attempts",
            container_id, target_addr, max_retries
        )))
    }

    /// Sets up a minimal container rootfs with a dummy service script.
    async fn setup_container_rootfs(
        &self,
        container_path: &str,
        service_config: &ServiceConfig,
    ) -> Result<(), ContainerError> {
        debug!("Setting up container rootfs at: {}", container_path);

        // Basic directory structure
        let dirs = [
            "bin",
            "usr/bin",
            "usr/local",
            "usr/local/bin",
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
        // Credentials: miel:miel
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

        // Create service script for the configuration
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
                    ContainerError::CreationFailed(format!("Failed to set script permissions: {}", e))
                })?;
        }

        debug!("Successfully set up container rootfs");
        Ok(())
    }

    /// Returns the command line to run for a given `service_config`.
    ///
    /// For SSH services, this includes comprehensive logging configuration to capture
    /// all session activity to the unified log file.
    fn get_service_command(&self, service_config: &ServiceConfig, host_port: u16, container_id: &str) -> String {
        // Get the log file path for this container (inside the container, the bind mount makes it accessible)
        let log_path = format!("/tmp/miel-logs/container-{}-activity.log", container_id);

        let command = match service_config.name.as_str() {
            "ssh" => {
                let p = host_port;
                // Enhanced SSH command with comprehensive logging for honeypot purposes
                format!(
                    r#"
                    # Create SSH host keys
                    /usr/bin/ssh-keygen -A >/dev/null 2>&1 || /bin/ssh-keygen -A >/dev/null 2>&1;

                    # Create a custom sshd_config for enhanced logging
                    cat > /etc/ssh/sshd_config << 'EOF'
Port {p}
ListenAddress 127.0.0.1
Protocol 2
HostKey /etc/ssh/ssh_host_rsa_key
HostKey /etc/ssh/ssh_host_ecdsa_key
HostKey /etc/ssh/ssh_host_ed25519_key
UsePrivilegeSeparation no
KeyRegenerationInterval 3600
ServerKeyBits 1024
SyslogFacility AUTH
LogLevel VERBOSE
LoginGraceTime 600
PermitRootLogin no
StrictModes yes
RSAAuthentication yes
PubkeyAuthentication yes
PasswordAuthentication yes
PermitEmptyPasswords no
ChallengeResponseAuthentication no
UsePAM no
X11Forwarding no
PrintMotd no
PrintLastLog yes
TCPKeepAlive yes
AcceptEnv LANG LC_*
Subsystem sftp /usr/libexec/openssh/sftp-server
UseDNS no
PidFile /var/run/sshd/sshd.pid
EOF

                    # Create a comprehensive logging shell for honeypot purposes
                    cat > /usr/local/bin/logged_shell << 'EOF'
#!/bin/sh
# Comprehensive honeypot shell logger - logs EVERYTHING including outputs
TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S UTC')
echo "[$TIMESTAMP] SSH session started for user: $USER (PID: $$)" >> {log_path}
echo "[$TIMESTAMP] [SSH-SESSION] Interactive shell session started" >> {log_path}

# Override PATH to intercept ALL command executions
export PATH="/tmp:$PATH"

# Create wrapper scripts for common binaries to log commands AND their outputs
for cmd in ls cat pwd whoami id ps top netstat ss w who uname find grep awk sed tail head more less vi nano wget curl chmod chown mkdir rmdir rm cp mv ln tar gzip gunzip file which env printenv history mount df du free uptime lscpu lsblk ifconfig ip route iptables nmap nc telnet ping traceroute dig nslookup; do
    if [ -x "/bin/$cmd" ] || [ -x "/usr/bin/$cmd" ] || [ -x "/sbin/$cmd" ] || [ -x "/usr/sbin/$cmd" ]; then
        cat > "/tmp/$cmd" << CMD_EOF
#!/bin/sh
TIMESTAMP=\$(date '+%Y-%m-%d %H:%M:%S UTC')
echo "[\$TIMESTAMP] [SSH-CMD] $cmd \$*" >> {log_path}

# Create temporary files for capturing output
OUTPUT_FILE="/tmp/cmd_output_\$\$"
ERROR_FILE="/tmp/cmd_error_\$\$"

# Find the real binary and execute it, capturing output
if [ -x "/bin/$cmd" ]; then
    "/bin/$cmd" "\$@" > "\$OUTPUT_FILE" 2> "\$ERROR_FILE"
elif [ -x "/usr/bin/$cmd" ]; then
    "/usr/bin/$cmd" "\$@" > "\$OUTPUT_FILE" 2> "\$ERROR_FILE"
elif [ -x "/sbin/$cmd" ]; then
    "/sbin/$cmd" "\$@" > "\$OUTPUT_FILE" 2> "\$ERROR_FILE"
elif [ -x "/usr/sbin/$cmd" ]; then
    "/usr/sbin/$cmd" "\$@" > "\$OUTPUT_FILE" 2> "\$ERROR_FILE"
else
    echo "$cmd: command not found" > "\$ERROR_FILE"
fi

EXIT_CODE=\$?
TIMESTAMP=\$(date '+%Y-%m-%d %H:%M:%S UTC')

# Log the output if there is any
if [ -s "\$OUTPUT_FILE" ]; then
    echo "[\$TIMESTAMP] [SSH-OUTPUT] $cmd stdout:" >> {log_path}
    while IFS= read -r line; do
        echo "[\$TIMESTAMP] [SSH-OUTPUT] \$line" >> {log_path}
    done < "\$OUTPUT_FILE"
fi

# Log any errors
if [ -s "\$ERROR_FILE" ]; then
    echo "[\$TIMESTAMP] [SSH-ERROR] $cmd stderr:" >> {log_path}
    while IFS= read -r line; do
        echo "[\$TIMESTAMP] [SSH-ERROR] \$line" >> {log_path}
    done < "\$ERROR_FILE"
fi

# Log the exit code
echo "[\$TIMESTAMP] [SSH-EXIT] $cmd exited with code: \$EXIT_CODE" >> {log_path}

# Display output to user using REAL binaries to avoid recursion
if [ -s "\$OUTPUT_FILE" ]; then
    if [ -x "/bin/cat" ]; then
        "/bin/cat" "\$OUTPUT_FILE"
    elif [ -x "/usr/bin/cat" ]; then
        "/usr/bin/cat" "\$OUTPUT_FILE"
    fi
fi

if [ -s "\$ERROR_FILE" ]; then
    if [ -x "/bin/cat" ]; then
        "/bin/cat" "\$ERROR_FILE" >&2
    elif [ -x "/usr/bin/cat" ]; then
        "/usr/bin/cat" "\$ERROR_FILE" >&2
    fi
fi

# Clean up temporary files using real rm to avoid recursion
if [ -x "/bin/rm" ]; then
    "/bin/rm" -f "\$OUTPUT_FILE" "\$ERROR_FILE" 2>/dev/null
elif [ -x "/usr/bin/rm" ]; then
    "/usr/bin/rm" -f "\$OUTPUT_FILE" "\$ERROR_FILE" 2>/dev/null
fi

exit \$EXIT_CODE
CMD_EOF
        chmod +x "/tmp/$cmd"
    fi
done

# Create a special wrapper for interactive commands that are harder to capture
for cmd in bash sh; do
    if [ -x "/bin/$cmd" ]; then
        cat > "/tmp/$cmd" << SHELL_EOF
#!/bin/sh
TIMESTAMP=\$(date '+%Y-%m-%d %H:%M:%S UTC')
echo "[\$TIMESTAMP] [SSH-CMD] $cmd \$*" >> {log_path}
echo "[\$TIMESTAMP] [SSH-WARNING] Interactive shell $cmd started - some commands may not be fully logged" >> {log_path}
exec "/bin/$cmd" "\$@"
SHELL_EOF
        chmod +x "/tmp/$cmd"
    fi
done

# Set up PS1 with command logging via DEBUG trap (bash feature) for comprehensive coverage
# If bash is available, use it for better command tracking
if [ -x "/bin/bash" ]; then
    export PS1='miel@honeypot:\w$ '
    exec /bin/bash --rcfile <(echo '
        set -o functrace
        shopt -s extdebug

        # Function to log commands with output capture
        log_bash_command() {{
            local cmd="$1"
            local timestamp=$(date "+%Y-%m-%d %H:%M:%S UTC")

            # Skip logging our own logging commands and temp file operations to avoid recursion
            case "$cmd" in
                *{log_path}*|*"date "*|*"echo "*timestamp*|*log_bash_command*|*"/tmp/cmd_output_"*|*"/tmp/cmd_error_"*|*"/bin/cat "*|*"/usr/bin/cat "*|*"/bin/rm "*|*"/usr/bin/rm "*)
                    return
                    ;;
            esac

            echo "[$timestamp] [SSH-CMD] $cmd" >> {log_path}
        }}

        # Set up DEBUG trap
        trap '\''log_bash_command "$BASH_COMMAND"'\'' DEBUG
        export PS1="miel@honeypot:\w$ "
    ')
else
    # Fallback to sh with custom PS1
    export PS1='miel@honeypot:$PWD$ '
    exec /bin/sh
fi
EOF
                    chmod +x /usr/local/bin/logged_shell

                    # Update the shell in passwd to use our logged shell for the miel user
                    sed -i 's|miel:x:1000:1000:miel User:/home/miel:/bin/sh|miel:x:1000:1000:miel User:/home/miel:/usr/local/bin/logged_shell|' /etc/passwd

                    # Start sshd with our custom config
                    exec /usr/sbin/sshd -D -e -f /etc/ssh/sshd_config 2>&1 | while IFS= read -r line; do
                        echo "[$(date '+%Y-%m-%d %H:%M:%S UTC')] [SSHD] $line" >> {log_path}
                    done
                    "#,
                    p = p,
                    log_path = log_path
                )
            }
            "http" => {
                let p = host_port;
                // Minimal HTTP server that just returns 200 OK
                format!(
                    r#"
                    # Set PATH to include all common Python locations
                    export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

                    # Try to find python3 in common locations and start minimal HTTP server
                    PYTHON3=""
                    for py in /usr/bin/python3 /usr/local/bin/python3 /bin/python3; do
                        if [ -x "$py" ]; then
                            PYTHON3="$py"
                            break
                        fi
                    done

                    if [ -z "$PYTHON3" ]; then
                        echo "[$(date '+%Y-%m-%d %H:%M:%S UTC')] [HTTP-ERROR] Python 3 not found in any standard location" >> {log_path}
                        exit 1
                    fi

                    echo "[$(date '+%Y-%m-%d %H:%M:%S UTC')] [HTTP-INFO] Starting minimal HTTP server with Python 3 at: $PYTHON3" >> {log_path}

                    # Start minimal HTTP server that just returns 200 OK
                    exec "$PYTHON3" -c "
import socket
import threading
import datetime

def handle_request(conn, addr):
    try:
        data = conn.recv(1024).decode('utf-8')
        # Simple 200 OK response
        response = 'HTTP/1.1 200 OK\\r\\nContent-Length: 2\\r\\n\\r\\nOK'
        conn.send(response.encode('utf-8'))

        # Log the request
        timestamp = datetime.datetime.now(datetime.timezone.utc).strftime('%Y-%m-%d %H:%M:%S UTC')
        with open('{log_path}', 'a') as f:
            f.write('[%s] [HTTP] Request from %s\\n' % (timestamp, addr[0]))
            f.flush()
    except:
        pass
    finally:
        conn.close()

# Create and bind socket
server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
server.bind(('127.0.0.1', {p}))
server.listen(5)

print('HTTP server listening on 127.0.0.1:{p}')

while True:
    conn, addr = server.accept()
    threading.Thread(target=handle_request, args=(conn, addr)).start()
" 2>&1 | while IFS= read -r line; do
    echo "[$(date '+%Y-%m-%d %H:%M:%S UTC')] [HTTP-SERVER] $line" >> {log_path}
done
                    "#,
                    p = p,
                    log_path = log_path
                )
            }
            _ => {
                format!(
                    r#"
                    # Generic service with logging
                    exec /bin/sh /usr/bin/service 2>&1 | while IFS= read -r line; do
                        echo "[$(date '+%Y-%m-%d %H:%M:%S UTC")] [SERVICE] $line" >> {log_path}
                    done
                    "#,
                    log_path = log_path
                )
            }
        };

        debug!(
            "Generated enhanced service command with logging for {}: {}",
            service_config.name, command.lines().take(3).collect::<Vec<_>>().join(" ")
        );
        command
    }

    /// Creates a unified log file to capture all shell activity from the container.
    ///
    /// This method creates a dedicated log file that will contain:
    /// - Main container shell output
    /// - SSH session activity (commands, output, interactions)
    /// - Any other shell/PTY activity within the container
    ///
    /// The log file is created with appropriate permissions and can be read
    /// to monitor all terminal activity happening inside the container.
    fn create_pty_master(&self, container_id: &str) -> Result<File, ContainerError> {
        // Create a dedicated log directory for container activity
        let log_dir = "/tmp/miel-logs";
        std::fs::create_dir_all(log_dir).map_err(|e| {
            error!("Failed to create log directory {}: {}", log_dir, e);
            ContainerError::CreationFailed(format!("Failed to create log directory: {}", e))
        })?;

        // Create a unified log file for all container shell activity
        let log_path = format!("{}/container-{}-activity.log", log_dir, container_id);
        debug!("Creating unified activity log at: {}", log_path);

        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&log_path)
            .map_err(|e| {
                error!("Failed to create activity log {}: {}", log_path, e);
                ContainerError::CreationFailed(format!("Failed to create activity log: {}", e))
            })?;

        // Set appropriate permissions to allow container to write
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = log_file.metadata()
                .map_err(|e| {
                    error!("Failed to get log file metadata: {}", e);
                    ContainerError::CreationFailed(format!("Failed to get log file metadata: {}", e))
                })?
                .permissions();
            perms.set_mode(0o666); // rw-rw-rw- - allow container to write
            std::fs::set_permissions(&log_path, perms)
                .map_err(|e| {
                    error!("Failed to set log file permissions: {}", e);
                    ContainerError::CreationFailed(format!("Failed to set log file permissions: {}", e))
                })?;
        }

        // Write initial log header
        use std::io::Write;
        let mut file_writer = &log_file;
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        writeln!(file_writer, "=== Container {} Activity Log Started at {} ===", container_id, timestamp)
            .map_err(|e| {
                error!("Failed to write log header: {}", e);
                ContainerError::CreationFailed(format!("Failed to write log header: {}", e))
            })?;
        file_writer.flush().map_err(|e| {
            error!("Failed to flush log file: {}", e);
            ContainerError::CreationFailed(format!("Failed to flush log file: {}", e))
        })?;

        info!("Successfully created unified activity log for container: {}", container_id);
        Ok(log_file)
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

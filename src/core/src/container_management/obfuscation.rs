use log::debug;
use std::fs;
use std::os::unix::fs::PermissionsExt;

use crate::configuration::types::{FakeProcess, ObfuscationConfig};
use crate::error_handling::types::ContainerError;

/// Handles obfuscation setup within containers to make them appear more realistic
pub struct ObfuscationManager;

impl ObfuscationManager {
    /// Sets up obfuscation artifacts in the container based on the configuration
    pub fn setup_obfuscation(
        container_path: &str,
        config: &ObfuscationConfig,
    ) -> Result<(), ContainerError> {
        if !config.enabled {
            debug!("Obfuscation disabled, skipping setup");
            return Ok(());
        }

        debug!(
            "Setting up obfuscation artifacts in container: {}",
            container_path
        );

        Self::setup_fake_hostname(container_path, config)?;
        Self::setup_fake_processes(container_path, config)?;
        Self::setup_fake_files(container_path, config)?;
        Self::setup_fake_users(container_path, config)?;
        Self::setup_fake_network(container_path, config)?;
        Self::setup_fake_uptime(container_path, config)?;

        debug!("Obfuscation setup completed for container");
        Ok(())
    }

    /// Sets up a fake hostname in the container
    fn setup_fake_hostname(
        container_path: &str,
        config: &ObfuscationConfig,
    ) -> Result<(), ContainerError> {
        if let Some(hostname) = &config.fake_hostname {
            debug!("Setting up fake hostname: {}", hostname);

            let hostname_path = format!("{}/etc/hostname", container_path);
            fs::write(&hostname_path, format!("{}\n", hostname)).map_err(|e| {
                ContainerError::CreationFailed(format!("Failed to write hostname file: {}", e))
            })?;

            // Also update /etc/hosts
            let hosts_path = format!("{}/etc/hosts", container_path);
            let hosts_content = format!(
                "127.0.0.1\tlocalhost\n127.0.1.1\t{}\n::1\tlocalhost ip6-localhost ip6-loopback\n",
                hostname
            );
            fs::write(&hosts_path, hosts_content).map_err(|e| {
                ContainerError::CreationFailed(format!("Failed to write hosts file: {}", e))
            })?;

            debug!("Fake hostname configured: {}", hostname);
        }
        Ok(())
    }

    /// Sets up fake processes that will appear in ps commands
    fn setup_fake_processes(
        container_path: &str,
        config: &ObfuscationConfig,
    ) -> Result<(), ContainerError> {
        if config.fake_processes.is_empty() {
            return Ok(());
        }

        debug!("Setting up {} fake processes", config.fake_processes.len());

        // Create a fake ps command that includes our processes
        let ps_script = Self::generate_fake_ps_script(&config.fake_processes);
        let ps_path = format!("{}/tmp/ps", container_path);

        fs::write(&ps_path, ps_script).map_err(|e| {
            ContainerError::CreationFailed(format!("Failed to create fake ps script: {}", e))
        })?;

        // Make it executable
        let mut perms = fs::metadata(&ps_path)
            .map_err(|e| {
                ContainerError::CreationFailed(format!("Failed to get ps script metadata: {}", e))
            })?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&ps_path, perms).map_err(|e| {
            ContainerError::CreationFailed(format!("Failed to set ps script permissions: {}", e))
        })?;

        // Also create fake top command
        let top_script = Self::generate_fake_top_script(&config.fake_processes);
        let top_path = format!("{}/tmp/top", container_path);

        fs::write(&top_path, top_script).map_err(|e| {
            ContainerError::CreationFailed(format!("Failed to create fake top script: {}", e))
        })?;

        let mut perms = fs::metadata(&top_path)
            .map_err(|e| {
                ContainerError::CreationFailed(format!("Failed to get top script metadata: {}", e))
            })?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&top_path, perms).map_err(|e| {
            ContainerError::CreationFailed(format!("Failed to set top script permissions: {}", e))
        })?;

        debug!("Fake process commands configured");
        Ok(())
    }

    /// Sets up fake files in the container
    fn setup_fake_files(
        container_path: &str,
        config: &ObfuscationConfig,
    ) -> Result<(), ContainerError> {
        if config.fake_files.is_empty() {
            return Ok(());
        }

        debug!("Setting up {} fake files", config.fake_files.len());

        for fake_file in &config.fake_files {
            let file_path = format!("{}{}", container_path, fake_file.path);

            // Create parent directories if needed
            if let Some(parent) = std::path::Path::new(&file_path).parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    ContainerError::CreationFailed(format!(
                        "Failed to create directory for fake file {}: {}",
                        fake_file.path, e
                    ))
                })?;
            }

            let content = fake_file
                .content
                .as_deref()
                .unwrap_or("# Fake file content\n");
            fs::write(&file_path, content).map_err(|e| {
                ContainerError::CreationFailed(format!(
                    "Failed to create fake file {}: {}",
                    fake_file.path, e
                ))
            })?;

            // Set executable permissions if needed
            if fake_file.is_executable {
                let mut perms = fs::metadata(&file_path)
                    .map_err(|e| {
                        ContainerError::CreationFailed(format!(
                            "Failed to get fake file metadata: {}",
                            e
                        ))
                    })?
                    .permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&file_path, perms).map_err(|e| {
                    ContainerError::CreationFailed(format!(
                        "Failed to set fake file permissions: {}",
                        e
                    ))
                })?;
            }

            debug!("Created fake file: {}", fake_file.path);
        }

        Ok(())
    }

    /// Sets up fake users in passwd/group files
    fn setup_fake_users(
        container_path: &str,
        config: &ObfuscationConfig,
    ) -> Result<(), ContainerError> {
        if config.fake_users.is_empty() {
            return Ok(());
        }

        debug!("Setting up {} fake users", config.fake_users.len());

        let passwd_path = format!("{}/etc/passwd", container_path);
        let group_path = format!("{}/etc/group", container_path);

        // Read existing content
        let mut passwd_content = fs::read_to_string(&passwd_path)
            .unwrap_or_else(|_| "root:x:0:0:root:/root:/bin/sh\n".to_string());
        let mut group_content =
            fs::read_to_string(&group_path).unwrap_or_else(|_| "root:x:0:\n".to_string());

        // Add fake users
        for (i, username) in config.fake_users.iter().enumerate() {
            let uid = 2000 + i as u32;
            passwd_content.push_str(&format!(
                "{}:x:{}:{}:{}:/home/{}:/bin/sh\n",
                username, uid, uid, username, username
            ));
            group_content.push_str(&format!("{}:x:{}:\n", username, uid));
        }

        fs::write(&passwd_path, passwd_content).map_err(|e| {
            ContainerError::CreationFailed(format!("Failed to update passwd file: {}", e))
        })?;

        fs::write(&group_path, group_content).map_err(|e| {
            ContainerError::CreationFailed(format!("Failed to update group file: {}", e))
        })?;

        debug!("Fake users configured");
        Ok(())
    }

    /// Sets up fake network interfaces
    fn setup_fake_network(
        container_path: &str,
        config: &ObfuscationConfig,
    ) -> Result<(), ContainerError> {
        if config.fake_network_interfaces.is_empty() {
            return Ok(());
        }

        debug!(
            "Setting up {} fake network interfaces",
            config.fake_network_interfaces.len()
        );

        // Create a fake ifconfig command
        let ifconfig_script = Self::generate_fake_ifconfig_script(&config.fake_network_interfaces);
        let ifconfig_path = format!("{}/tmp/ifconfig", container_path);

        fs::write(&ifconfig_path, ifconfig_script).map_err(|e| {
            ContainerError::CreationFailed(format!("Failed to create fake ifconfig script: {}", e))
        })?;

        let mut perms = fs::metadata(&ifconfig_path)
            .map_err(|e| {
                ContainerError::CreationFailed(format!(
                    "Failed to get ifconfig script metadata: {}",
                    e
                ))
            })?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&ifconfig_path, perms).map_err(|e| {
            ContainerError::CreationFailed(format!(
                "Failed to set ifconfig script permissions: {}",
                e
            ))
        })?;

        // Also create fake ip command
        let ip_script = Self::generate_fake_ip_script(&config.fake_network_interfaces);
        let ip_path = format!("{}/tmp/ip", container_path);

        fs::write(&ip_path, ip_script).map_err(|e| {
            ContainerError::CreationFailed(format!("Failed to create fake ip script: {}", e))
        })?;

        let mut perms = fs::metadata(&ip_path)
            .map_err(|e| {
                ContainerError::CreationFailed(format!("Failed to get ip script metadata: {}", e))
            })?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&ip_path, perms).map_err(|e| {
            ContainerError::CreationFailed(format!("Failed to set ip script permissions: {}", e))
        })?;

        debug!("Fake network interfaces configured");
        Ok(())
    }

    /// Sets up fake system uptime
    fn setup_fake_uptime(
        container_path: &str,
        config: &ObfuscationConfig,
    ) -> Result<(), ContainerError> {
        if let Some(uptime_days) = config.system_uptime_days {
            debug!("Setting up fake uptime: {} days", uptime_days);

            let uptime_script = format!(
                r#"#!/bin/sh
# Fake uptime command
UPTIME_DAYS={}
UPTIME_HOURS=$((UPTIME_DAYS * 24))
UPTIME_MINUTES=$((UPTIME_HOURS * 60))
echo " $(date '+%H:%M:%S') up $UPTIME_DAYS days, $(($UPTIME_HOURS % 24)):$(($UPTIME_MINUTES % 60)), 2 users, load average: 0.15, 0.25, 0.18"
"#,
                uptime_days
            );

            let uptime_path = format!("{}/tmp/uptime", container_path);
            fs::write(&uptime_path, uptime_script).map_err(|e| {
                ContainerError::CreationFailed(format!(
                    "Failed to create fake uptime script: {}",
                    e
                ))
            })?;

            let mut perms = fs::metadata(&uptime_path)
                .map_err(|e| {
                    ContainerError::CreationFailed(format!(
                        "Failed to get uptime script metadata: {}",
                        e
                    ))
                })?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&uptime_path, perms).map_err(|e| {
                ContainerError::CreationFailed(format!(
                    "Failed to set uptime script permissions: {}",
                    e
                ))
            })?;

            debug!("Fake uptime configured");
        }
        Ok(())
    }

    /// Generates a fake ps script that includes configured processes
    fn generate_fake_ps_script(processes: &[FakeProcess]) -> String {
        let mut script = String::from("#!/bin/sh\n# Fake ps command\necho 'PID TTY TIME CMD'\n");

        // Add some basic system processes
        script.push_str("echo '  1 ?   00:00:01 systemd'\n");
        script.push_str("echo '  2 ?   00:00:00 kthreadd'\n");

        // Add configured fake processes
        for (i, process) in processes.iter().enumerate() {
            let pid = process.pid.unwrap_or(100 + i as u32);
            script.push_str(&format!(
                "echo '{:3} ?   00:00:0{} {}'\n",
                pid,
                i % 10,
                process.name
            ));
        }

        script
    }

    /// Generates a fake top script that shows configured processes
    fn generate_fake_top_script(processes: &[FakeProcess]) -> String {
        let mut script = String::from(
            r#"#!/bin/sh
# Fake top command
echo "top - $(date '+%H:%M:%S') up 1 day, 2:34, 2 users, load average: 0.15, 0.25, 0.18"
echo "Tasks: $(expr 5 + $#) total, 1 running, $(expr 4 + $#) sleeping, 0 stopped, 0 zombie"
echo "Cpu(s): 2.3%us, 1.2%sy, 0.0%ni, 96.1%id, 0.4%wa, 0.0%hi, 0.0%si, 0.0%st"
echo "Mem: 2048000k total, 1024000k used, 1024000k free, 64000k buffers"
echo "Swap: 1048576k total, 0k used, 1048576k free, 512000k cached"
echo ""
echo "  PID USER      PR  NI  VIRT  RES  SHR S %CPU %MEM    TIME+  COMMAND"
echo "    1 root      20   0 19232 1464 1172 S  0.0  0.1   0:01.23 systemd"
"#,
        );

        // Add configured fake processes
        for (i, process) in processes.iter().enumerate() {
            let pid = process.pid.unwrap_or(100 + i as u32);
            let cpu = process.cpu_percent.unwrap_or(0.1 + (i as f32 * 0.3));
            let mem_mb = process.memory_mb.unwrap_or(10 + i as u32 * 5);

            script.push_str(&format!(
                "echo '{:5} root      20   0 {:5} {:4} {:4} S {:4.1} {:4.1}   0:0{:02}.{:02} {}'\n",
                pid,
                mem_mb * 1024,
                mem_mb,
                mem_mb / 2,
                cpu,
                (mem_mb as f32 / 2048000.0) * 100.0,
                i % 60,
                i % 100,
                process.name
            ));
        }

        script
    }

    /// Generates a fake ifconfig script
    fn generate_fake_ifconfig_script(interfaces: &[String]) -> String {
        let mut script = String::from("#!/bin/sh\n# Fake ifconfig command\n");

        // Always show loopback
        script.push_str(
            r#"echo "lo        Link encap:Local Loopback"
echo "          inet addr:127.0.0.1  Mask:255.0.0.0"
echo "          UP LOOPBACK RUNNING  MTU:65536  Metric:1"
echo ""
"#,
        );

        // Add fake interfaces
        for (i, interface) in interfaces.iter().enumerate() {
            let ip_suffix = 100 + i;
            script.push_str(&format!(
                r#"echo "{interface}      Link encap:Ethernet  HWaddr 02:42:ac:11:00:{i:02x}"
echo "          inet addr:192.168.1.{ip_suffix}  Bcast:192.168.1.255  Mask:255.255.255.0"
echo "          UP BROADCAST RUNNING MULTICAST  MTU:1500  Metric:1"
echo ""
"#,
                interface = interface,
                i = i,
                ip_suffix = ip_suffix
            ));
        }

        script
    }

    /// Generates a fake ip script
    fn generate_fake_ip_script(interfaces: &[String]) -> String {
        let mut script = String::from("#!/bin/sh\n# Fake ip command\n");

        if interfaces.is_empty() {
            script.push_str(
                "echo '1: lo: <LOOPBACK,UP,LOWER_UP> mtu 65536 qdisc noqueue state UNKNOWN'\n",
            );
            script.push_str("echo '    inet 127.0.0.1/8 scope host lo'\n");
            return script;
        }

        script.push_str(
            "echo '1: lo: <LOOPBACK,UP,LOWER_UP> mtu 65536 qdisc noqueue state UNKNOWN'\n",
        );
        script.push_str("echo '    inet 127.0.0.1/8 scope host lo'\n");

        for (i, interface) in interfaces.iter().enumerate() {
            let ip_suffix = 100 + i;
            let if_index = i + 2;
            script.push_str(&format!(
                "echo '{}: {}: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 qdisc pfifo_fast state UP'\n",
                if_index, interface
            ));
            script.push_str(&format!(
                "echo '    inet 192.168.1.{}/24 brd 192.168.1.255 scope global {}'\n",
                ip_suffix, interface
            ));
        }

        script
    }
}

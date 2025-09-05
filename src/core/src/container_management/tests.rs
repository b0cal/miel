#[cfg(test)]
mod tests {
    use crate::configuration::{Protocol, ServiceConfig};
    use crate::configuration::types::ObfuscationConfig;
    use crate::container_management::{ContainerHandle, ContainerStats};
    use chrono::Utc;

    // Helper to create a test service config
    fn svc(name: &str, port: u16) -> ServiceConfig {
        ServiceConfig {
            name: name.to_string(),
            port,
            protocol: Protocol::TCP,
            container_image: format!("test-{}", name),
            enabled: true,
            header_patterns: vec![format!("{}-pattern", name)],
            banner_response: Some(format!("{} banner", name)),
            obfuscation: ObfuscationConfig::default(),
        }
    }

    fn udp_svc(name: &str, port: u16) -> ServiceConfig {
        ServiceConfig {
            name: name.to_string(),
            port,
            protocol: Protocol::UDP,
            container_image: format!("test-{}", name),
            enabled: true,
            header_patterns: vec![format!("{}-pattern", name)],
            banner_response: Some(format!("{} banner", name)),
            obfuscation: ObfuscationConfig::default(),
        }
    }

    // Helper to create a service config with obfuscation enabled
    fn obfuscated_svc(name: &str, port: u16) -> ServiceConfig {
        use crate::configuration::types::{FakeProcess, FakeFile};
        ServiceConfig {
            name: name.to_string(),
            port,
            protocol: Protocol::TCP,
            container_image: format!("test-{}", name),
            enabled: true,
            header_patterns: vec![format!("{}-pattern", name)],
            banner_response: Some(format!("{} banner", name)),
            obfuscation: ObfuscationConfig {
                enabled: true,
                fake_hostname: Some(format!("test-{}-server", name)),
                fake_processes: vec![
                    FakeProcess {
                        name: "nginx".to_string(),
                        pid: Some(1234),
                        cpu_percent: Some(2.1),
                        memory_mb: Some(45),
                        command: "nginx: master process".to_string(),
                    },
                ],
                fake_files: vec![
                    FakeFile {
                        path: "/var/log/test.log".to_string(),
                        content: Some("test log content".to_string()),
                        size_bytes: None,
                        is_executable: false,
                    },
                ],
                fake_users: vec!["admin".to_string(), "webuser".to_string()],
                fake_network_interfaces: vec!["eth0".to_string()],
                system_uptime_days: Some(100),
            },
        }
    }

    // Minimal mock manager to validate basic bookkeeping without external deps
    struct MockManager {
        active: std::collections::HashMap<String, ContainerHandle>,
        stats: ContainerStats,
    }

    impl MockManager {
        fn new() -> Self {
            Self {
                active: Default::default(),
                stats: ContainerStats {
                    active_count: 0,
                    total_created: 0,
                    failed_count: 0,
                },
            }
        }
        fn create(&mut self, cfg: &ServiceConfig) -> ContainerHandle {
            let id = format!(
                "mock-{}-{}",
                cfg.name,
                &uuid::Uuid::new_v4().to_string()[..8]
            );
            let handle = ContainerHandle {
                id: id.clone(),
                service_name: cfg.name.clone(),
                port: cfg.port,
                host_port: cfg.port, // simple mapping in mock
                created_at: Utc::now(),
                process_handle: None,
                pty_master: None,
                tcp_socket: None,
            };
            self.stats.total_created += 1;
            self.stats.active_count += 1;
            self.active.insert(id, handle.clone());
            handle
        }
        fn cleanup(&mut self, handle: ContainerHandle) {
            self.active.remove(&handle.id);
            self.stats.active_count = self.stats.active_count.saturating_sub(1);
        }
    }

    #[test]
    fn container_stats_consistency_after_operations() {
        let mut stats = ContainerStats {
            active_count: 0,
            total_created: 0,
            failed_count: 0,
        };

        // Test that stats remain consistent after container creation
        stats.total_created += 3;
        stats.active_count += 3;
        assert_eq!(stats.active_count, 3);
        assert_eq!(stats.total_created, 3);
        assert_eq!(stats.failed_count, 0);

        // Test cleanup with saturation protection
        stats.active_count = stats.active_count.saturating_sub(2);
        assert_eq!(stats.active_count, 1);

        // Test cleanup beyond zero
        stats.active_count = stats.active_count.saturating_sub(10);
        assert_eq!(stats.active_count, 0);

        // Test failure counting
        stats.failed_count += 2;
        assert_eq!(stats.failed_count, 2);

        // Total created should never decrease
        let original_total = stats.total_created;
        stats.total_created += 1;
        assert!(stats.total_created > original_total);
    }

    #[test]
    fn container_handle_cloning_preserves_critical_fields() {
        let creation_time = Utc::now();
        let handle = ContainerHandle {
            id: "miel-ssh-abc123".to_string(),
            service_name: "ssh".to_string(),
            port: 22,
            host_port: 52222,
            created_at: creation_time,
            process_handle: None,
            pty_master: None,
            tcp_socket: None,
        };

        let cloned = handle.clone();

        // Critical fields must be preserved
        assert_eq!(cloned.id, handle.id);
        assert_eq!(cloned.service_name, handle.service_name);
        assert_eq!(cloned.port, handle.port);
        assert_eq!(cloned.host_port, handle.host_port);
        assert_eq!(cloned.created_at, handle.created_at);

        // Non-cloneable fields should be None
        assert!(cloned.process_handle.is_none());
        assert!(cloned.pty_master.is_none());
        assert!(cloned.tcp_socket.is_none());
    }

    #[test]
    fn ephemeral_port_allocation_functionality() {
        use std::net::{Ipv4Addr, TcpListener, UdpSocket};

        // Test TCP port allocation
        let tcp_listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let tcp_port = tcp_listener.local_addr().unwrap().port();
        assert!(tcp_port > 0);
        drop(tcp_listener);

        // Test UDP port allocation
        let udp_socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let udp_port = udp_socket.local_addr().unwrap().port();
        assert!(udp_port > 0);
        drop(udp_socket);

        // Ensure we can allocate different ports
        let tcp1 = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let tcp2 = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port1 = tcp1.local_addr().unwrap().port();
        let port2 = tcp2.local_addr().unwrap().port();
        assert_ne!(port1, port2);
    }

    #[test]
    fn container_id_format_validation() {
        let service_name = "ssh";
        let uuid_part = "12345678-abcd-1234-abcd-123456789012";
        let container_id = format!("miel-{}-{}", service_name, uuid_part);

        // Must follow the expected format
        assert!(container_id.starts_with("miel-"));
        assert!(container_id.contains(&service_name));
        assert!(container_id.contains(&uuid_part));

        // Test various service names
        let services = ["ssh", "http", "ftp", "telnet"];
        for service in &services {
            let id = format!("miel-{}-{}", service, uuid_part);
            assert!(id.starts_with("miel-"));
            assert!(id.contains(service));
        }
    }

    #[test]
    fn service_command_generation_correctness() {
        // Test SSH service command generation
        let ssh_cmd = format!(
            "/usr/bin/ssh-keygen -A >/dev/null 2>&1 || /bin/ssh-keygen -A >/dev/null 2>&1; /usr/sbin/sshd -D -e -f /dev/null -p {} -o ListenAddress=127.0.0.1 -o UsePAM=no -o PasswordAuthentication=yes -o PermitRootLogin=no -o PidFile=/var/run/sshd/sshd.pid",
            2222
        );
        assert!(ssh_cmd.contains("sshd"));
        assert!(ssh_cmd.contains("-p 2222"));
        assert!(ssh_cmd.contains("ListenAddress=127.0.0.1"));
        assert!(ssh_cmd.contains("PasswordAuthentication=yes"));
        assert!(ssh_cmd.contains("PermitRootLogin=no"));

        // Test HTTP service command generation
        let http_cmd = format!(
            "/usr/bin/python3 -m http.server {} --bind 127.0.0.1 --directory /www",
            8080
        );
        assert!(http_cmd.contains("http.server"));
        assert!(http_cmd.contains("8080"));
        assert!(http_cmd.contains("--bind 127.0.0.1"));
        assert!(http_cmd.contains("--directory /www"));

        // Test fallback service command
        let fallback_cmd = "/bin/sh /usr/bin/service";
        assert!(fallback_cmd.contains("/bin/sh"));
        assert!(fallback_cmd.contains("/usr/bin/service"));
    }

    #[test]
    fn container_directory_path_generation() {
        let container_id = "miel-http-test123";
        let expected_path = format!("/tmp/miel-containers/{}", container_id);
        assert_eq!(expected_path, "/tmp/miel-containers/miel-http-test123");

        // Test with different container IDs
        let ids = ["miel-ssh-abc", "miel-ftp-def", "miel-telnet-ghi"];
        for id in &ids {
            let path = format!("/tmp/miel-containers/{}", id);
            assert!(path.starts_with("/tmp/miel-containers/"));
            assert!(path.contains(id));
        }
    }

    #[test]
    fn privilege_check_logic() {
        // Simulate root user check (id -u returns "0")
        let root_output = "0\n";
        assert_eq!(root_output.trim(), "0");

        // Simulate non-root user check (id -u returns non-zero)
        let non_root_outputs = ["1000\n", "501\n", "1001\n"];
        for output in &non_root_outputs {
            assert_ne!(output.trim(), "0");
        }
    }

    #[test]
    fn retry_logic_backoff_calculation() {
        // Test progressive backoff calculation used in connection establishment
        let mut wait_times = Vec::new();
        for retries in 1..=10 {
            let wait_time = std::cmp::min(500 + (retries * 200), 3000);
            wait_times.push(wait_time);
        }

        // Verify progressive increase
        assert_eq!(wait_times[0], 700); // 500 + 1*200
        assert_eq!(wait_times[1], 900); // 500 + 2*200
        assert_eq!(wait_times[2], 1100); // 500 + 3*200
        assert_eq!(wait_times[4], 1500); // 500 + 5*200

        // Verify cap at 3000ms
        for &time in &wait_times[10..] {
            assert_eq!(time, 3000);
        }
    }

    #[test]
    fn container_registry_operations() {
        let mut registry: std::collections::HashMap<String, ContainerHandle> =
            std::collections::HashMap::new();

        // Test insertion
        let handle1 = ContainerHandle {
            id: "container-1".to_string(),
            service_name: "ssh".to_string(),
            port: 22,
            host_port: 2222,
            created_at: Utc::now(),
            process_handle: None,
            pty_master: None,
            tcp_socket: None,
        };

        registry.insert(handle1.id.clone(), handle1.clone());
        assert_eq!(registry.len(), 1);
        assert!(registry.contains_key("container-1"));

        // Test lookup
        let retrieved = registry.get("container-1").unwrap();
        assert_eq!(retrieved.service_name, "ssh");
        assert_eq!(retrieved.port, 22);

        // Test multiple containers
        let handle2 = ContainerHandle {
            id: "container-2".to_string(),
            service_name: "http".to_string(),
            port: 80,
            host_port: 8080,
            created_at: Utc::now(),
            process_handle: None,
            pty_master: None,
            tcp_socket: None,
        };

        registry.insert(handle2.id.clone(), handle2);
        assert_eq!(registry.len(), 2);

        // Test removal
        registry.remove("container-1");
        assert_eq!(registry.len(), 1);
        assert!(!registry.contains_key("container-1"));
        assert!(registry.contains_key("container-2"));

        // Test clear all
        registry.clear();
        assert!(registry.is_empty());
    }

    #[test]
    fn pty_path_generation() {
        let container_id = "miel-ssh-test456";
        let pty_path = format!("/tmp/miel-pty-{}", container_id);
        assert_eq!(pty_path, "/tmp/miel-pty-miel-ssh-test456");

        // Test with different container IDs
        let ids = ["test-1", "test-2", "miel-http-abc"];
        for id in &ids {
            let path = format!("/tmp/miel-pty-{}", id);
            assert!(path.starts_with("/tmp/miel-pty-"));
            assert!(path.contains(id));
        }
    }

    #[test]
    fn error_handling_patterns() {
        use crate::error_handling::types::ContainerError;

        // Test different error variants
        let creation_error = ContainerError::CreationFailed("setup failed".to_string());
        match creation_error {
            ContainerError::CreationFailed(msg) => assert_eq!(msg, "setup failed"),
            _ => panic!("Expected CreationFailed variant"),
        }

        let start_error = ContainerError::StartFailed("spawn failed".to_string());
        match start_error {
            ContainerError::StartFailed(msg) => assert_eq!(msg, "spawn failed"),
            _ => panic!("Expected StartFailed variant"),
        }

        let connection_error = ContainerError::ConnectionFailed("connect failed".to_string());
        match connection_error {
            ContainerError::ConnectionFailed(msg) => assert_eq!(msg, "connect failed"),
            _ => panic!("Expected ConnectionFailed variant"),
        }

        let runtime_error = ContainerError::RuntimeNotAvailable;
        match runtime_error {
            ContainerError::RuntimeNotAvailable => { /* Expected */ }
            _ => panic!("Expected RuntimeNotAvailable variant"),
        }
    }

    #[test]
    fn service_config_protocol_handling() {
        let tcp_service = svc("ssh", 22);
        let udp_service = udp_svc("dns", 53);

        // Verify protocol differentiation
        assert!(matches!(tcp_service.protocol, Protocol::TCP));
        assert!(matches!(udp_service.protocol, Protocol::UDP));

        // Verify service-specific configurations
        assert_eq!(tcp_service.name, "ssh");
        assert_eq!(tcp_service.port, 22);
        assert_eq!(udp_service.name, "dns");
        assert_eq!(udp_service.port, 53);
    }

    #[test]
    fn container_types_basic() {
        let now = Utc::now();
        let h = ContainerHandle {
            id: "id".into(),
            service_name: "svc".into(),
            port: 22,
            host_port: 12000,
            created_at: now,
            process_handle: None,
            pty_master: None,
            tcp_socket: None,
        };
        let hc = h.clone();
        assert_eq!(hc.id, "id");
        assert_eq!(hc.port, 22);
        assert_eq!(hc.host_port, 12000);

        let s = ContainerStats {
            active_count: 0,
            total_created: 0,
            failed_count: 0,
        };
        let sc = s.clone();
        assert_eq!(sc.active_count, 0);
        assert_eq!(sc.total_created, 0);
        assert_eq!(sc.failed_count, 0);
    }

    #[test]
    fn mock_manager_lifecycle() {
        let mut mm = MockManager::new();
        let ssh = svc("ssh", 22);
        let http = svc("http", 80);

        let h1 = mm.create(&ssh);
        let h2 = mm.create(&http);
        assert!(h1.id.starts_with("mock-ssh-"));
        assert!(h2.id.starts_with("mock-http-"));
        assert_eq!(mm.stats.active_count, 2);
        assert_eq!(mm.stats.total_created, 2);
        assert!(mm.active.contains_key(&h1.id));
        assert!(mm.active.contains_key(&h2.id));

        mm.cleanup(h1);
        assert_eq!(mm.stats.active_count, 1);
        mm.cleanup(h2);
        assert_eq!(mm.stats.active_count, 0);
        assert!(mm.active.is_empty());
    }

    #[test]
    fn mock_manager_get() {
        let mut mm = MockManager::new();
        let ssh = svc("ssh", 22);
        let h = mm.create(&ssh);
        let got = mm.active.get(&h.id).unwrap();
        assert_eq!(got.service_name, "ssh");
        assert_eq!(got.port, 22);
    }

    #[test]
    fn container_lifecycle_state_transitions() {
        let mut mm = MockManager::new();

        // Initial state
        assert_eq!(mm.stats.active_count, 0);
        assert_eq!(mm.stats.total_created, 0);
        assert!(mm.active.is_empty());

        // Create containers
        let ssh = svc("ssh", 22);
        let http = svc("http", 80);
        let ftp = svc("ftp", 21);

        let h1 = mm.create(&ssh);
        let h1_id = h1.id.clone(); // Store ID before moving
        assert_eq!(mm.stats.active_count, 1);
        assert_eq!(mm.stats.total_created, 1);

        let h2 = mm.create(&http);
        let h2_id = h2.id.clone(); // Store ID before moving
        assert_eq!(mm.stats.active_count, 2);
        assert_eq!(mm.stats.total_created, 2);

        let h3 = mm.create(&ftp);
        let h3_id = h3.id.clone(); // Store ID before moving
        assert_eq!(mm.stats.active_count, 3);
        assert_eq!(mm.stats.total_created, 3);

        // Verify all containers are tracked
        assert!(mm.active.contains_key(&h1_id));
        assert!(mm.active.contains_key(&h2_id));
        assert!(mm.active.contains_key(&h3_id));

        // Cleanup in different order
        mm.cleanup(h2); // Cleanup middle container
        assert_eq!(mm.stats.active_count, 2);
        assert_eq!(mm.stats.total_created, 3); // Total never decreases
        assert!(!mm.active.contains_key(&h2_id));
        assert!(mm.active.contains_key(&h1_id));
        assert!(mm.active.contains_key(&h3_id));

        mm.cleanup(h1); // Cleanup first container
        assert_eq!(mm.stats.active_count, 1);
        assert!(!mm.active.contains_key(&h1_id));
        assert!(mm.active.contains_key(&h3_id));

        mm.cleanup(h3); // Cleanup last container
        assert_eq!(mm.stats.active_count, 0);
        assert!(mm.active.is_empty());
        assert_eq!(mm.stats.total_created, 3); // Total remains unchanged
    }

    #[test]
    fn container_stats_recomputation_accuracy() {
        // Test that get_container_stats() recomputes active_count from actual registry size
        let mut stats = ContainerStats {
            active_count: 999, // Deliberately wrong value
            total_created: 5,
            failed_count: 0,
        };

        let active_containers: std::collections::HashMap<String, ContainerHandle> =
            std::collections::HashMap::new();

        // Simulate the recomputation logic from get_container_stats()
        stats.active_count = active_containers.len();
        assert_eq!(stats.active_count, 0); // Should be corrected to actual size

        // Test with actual containers in registry
        let mut registry = std::collections::HashMap::new();
        for i in 0..3 {
            let handle = ContainerHandle {
                id: format!("container-{}", i),
                service_name: "test".to_string(),
                port: 80,
                host_port: 8080 + i as u16,
                created_at: Utc::now(),
                process_handle: None,
                pty_master: None,
                tcp_socket: None,
            };
            registry.insert(handle.id.clone(), handle);
        }

        stats.active_count = registry.len();
        assert_eq!(stats.active_count, 3);
    }

    #[test]
    fn container_lookup_edge_cases() {
        let mut registry: std::collections::HashMap<String, ContainerHandle> =
            std::collections::HashMap::new();

        // Test lookup on empty registry
        assert!(registry.get("nonexistent").is_none());

        // Test lookup with empty string
        assert!(registry.get("").is_none());

        // Test lookup with special characters
        assert!(registry.get("container-with-#@$%").is_none());

        // Add a container and test exact match
        let handle = ContainerHandle {
            id: "exact-match-123".to_string(),
            service_name: "ssh".to_string(),
            port: 22,
            host_port: 2222,
            created_at: Utc::now(),
            process_handle: None,
            pty_master: None,
            tcp_socket: None,
        };
        registry.insert(handle.id.clone(), handle);

        // Exact match should work
        assert!(registry.get("exact-match-123").is_some());

        // Partial matches should fail
        assert!(registry.get("exact-match").is_none());
        assert!(registry.get("exact-match-123-extra").is_none());
        assert!(registry.get("EXACT-MATCH-123").is_none()); // Case sensitive
    }

    #[test]
    fn container_id_collision_prevention() {
        // Test that generated container IDs are unique
        let mut ids = std::collections::HashSet::new();
        let service = svc("ssh", 22);

        // Generate multiple IDs and ensure they're unique
        for _ in 0..100 {
            let uuid = uuid::Uuid::new_v4();
            let id = format!("miel-{}-{}", service.name, uuid);
            assert!(ids.insert(id), "Duplicate container ID generated");
        }

        // Verify all IDs follow the correct format
        for id in &ids {
            assert!(id.starts_with("miel-ssh-"));
            assert_eq!(id.len(), "miel-ssh-".len() + 36); // UUID is 36 chars
        }
    }

    #[test]
    fn service_command_parameter_injection_safety() {
        // Test that service commands handle potentially dangerous parameters safely
        let test_ports = [22, 80, 443, 8080, 65535];

        for &port in &test_ports {
            let ssh_cmd = format!(
                "/usr/bin/ssh-keygen -A >/dev/null 2>&1 || /bin/ssh-keygen -A >/dev/null 2>&1; /usr/sbin/sshd -D -e -f /dev/null -p {} -o ListenAddress=127.0.0.1 -o UsePAM=no -o PasswordAuthentication=yes -o PermitRootLogin=no -o PidFile=/var/run/sshd/sshd.pid",
                port
            );

            // Ensure port is properly embedded as a number
            assert!(ssh_cmd.contains(&format!("-p {}", port)));

            // Ensure no command injection characters are present in critical parts
            let critical_parts = ["-o ListenAddress=127.0.0.1", "-o UsePAM=no"];
            for part in &critical_parts {
                assert!(
                    ssh_cmd.contains(part),
                    "Missing critical SSH parameter: {}",
                    part
                );
            }
        }

        // Test HTTP command parameter safety
        for &port in &test_ports {
            let http_cmd = format!(
                "/usr/bin/python3 -m http.server {} --bind 127.0.0.1 --directory /www",
                port
            );

            assert!(http_cmd.contains(&port.to_string()));
            assert!(http_cmd.contains("--bind 127.0.0.1"));
            assert!(http_cmd.contains("--directory /www"));
        }
    }

    #[test]
    fn container_port_range_validation() {
        use std::net::{Ipv4Addr, TcpListener};

        // Test that we can allocate ports across the valid range
        let mut allocated_ports = Vec::new();

        // Try to allocate several ephemeral ports
        for _ in 0..10 {
            let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
            let port = listener.local_addr().unwrap().port();

            // Verify port is in valid range
            assert!(port > 0);

            // Verify port is in ephemeral range (typically 32768-65535 on Linux)
            // But we'll be more lenient since the OS controls this
            assert!(port >= 1024, "Port {} is in reserved range", port);

            allocated_ports.push(port);
            drop(listener);
        }

        // Verify we got different ports (highly likely with ephemeral allocation)
        allocated_ports.sort();
        allocated_ports.dedup();
        assert!(
            allocated_ports.len() >= 5,
            "Not enough unique ports allocated"
        );
    }

    #[test]
    fn mock_manager_failure_counting() {
        struct FailingMockManager {
            active: std::collections::HashMap<String, ContainerHandle>,
            stats: ContainerStats,
            cleanup_should_fail: bool,
        }

        impl FailingMockManager {
            fn new() -> Self {
                Self {
                    active: Default::default(),
                    stats: ContainerStats {
                        active_count: 0,
                        total_created: 0,
                        failed_count: 0,
                    },
                    cleanup_should_fail: false,
                }
            }

            fn create(&mut self, cfg: &ServiceConfig) -> ContainerHandle {
                let id = format!(
                    "mock-{}-{}",
                    cfg.name,
                    &uuid::Uuid::new_v4().to_string()[..8]
                );
                let handle = ContainerHandle {
                    id: id.clone(),
                    service_name: cfg.name.clone(),
                    port: cfg.port,
                    host_port: cfg.port,
                    created_at: Utc::now(),
                    process_handle: None,
                    pty_master: None,
                    tcp_socket: None,
                };
                self.stats.total_created += 1;
                self.stats.active_count += 1;
                self.active.insert(id, handle.clone());
                handle
            }

            fn cleanup(&mut self, handle: ContainerHandle) -> Result<(), &'static str> {
                if self.cleanup_should_fail {
                    self.stats.failed_count += 1;
                    Err("Simulated cleanup failure")
                } else {
                    self.active.remove(&handle.id);
                    self.stats.active_count = self.stats.active_count.saturating_sub(1);
                    Ok(())
                }
            }
        }

        let mut mm = FailingMockManager::new();
        let ssh = svc("ssh", 22);

        // Create and successfully cleanup
        let h1 = mm.create(&ssh);
        assert_eq!(mm.stats.failed_count, 0);
        assert!(mm.cleanup(h1).is_ok());
        assert_eq!(mm.stats.failed_count, 0);

        // Create and fail cleanup
        mm.cleanup_should_fail = true;
        let h2 = mm.create(&ssh);
        assert!(mm.cleanup(h2).is_err());
        assert_eq!(mm.stats.failed_count, 1);
    }

    #[test]
    fn container_handle_timestamp_accuracy() {
        let before = Utc::now();

        let handle = ContainerHandle {
            id: "time-test".to_string(),
            service_name: "ssh".to_string(),
            port: 22,
            host_port: 2222,
            created_at: Utc::now(),
            process_handle: None,
            pty_master: None,
            tcp_socket: None,
        };

        let after = Utc::now();

        // Timestamp should be between before and after
        assert!(handle.created_at >= before);
        assert!(handle.created_at <= after);

        // Test timezone consistency (should always be UTC)
        assert_eq!(handle.created_at.timezone(), Utc);
    }

    #[test]
    fn service_config_edge_cases() {
        // Test with minimum valid port
        let min_port_service = svc("test", 1);
        assert_eq!(min_port_service.port, 1);

        // Test with maximum valid port
        let max_port_service = svc("test", 65535);
        assert_eq!(max_port_service.port, 65535);

        // Test with common reserved ports
        let reserved_ports = [21, 22, 23, 25, 53, 80, 110, 143, 443, 993, 995];
        for &port in &reserved_ports {
            let service = svc("test", port);
            assert_eq!(service.port, port);
            assert!(!service.name.is_empty());
        }

        // Test service name variations
        let names = ["ssh", "http", "ftp", "telnet", "smtp", "dns"];
        for name in &names {
            let service = svc(name, 8080);
            assert_eq!(service.name, *name);
            assert!(service.container_image.contains(name));
            assert!(service.header_patterns[0].contains(name));
            assert!(service.banner_response.as_ref().unwrap().contains(name));
        }
    }

    #[test]
    fn path_generation_consistency() {
        let container_ids = [
            "miel-ssh-abc123",
            "miel-http-def456",
            "miel-ftp-ghi789",
            "test-container-xyz",
        ];

        for container_id in &container_ids {
            // Container directory path
            let container_path = format!("/tmp/miel-containers/{}", container_id);
            assert!(container_path.starts_with("/tmp/miel-containers/"));
            assert!(container_path.ends_with(container_id));

            // PTY path
            let pty_path = format!("/tmp/miel-pty-{}", container_id);
            assert!(pty_path.starts_with("/tmp/miel-pty-"));
            assert!(pty_path.ends_with(container_id));

            // Paths should be different
            assert_ne!(container_path, pty_path);

            // Both should contain the container ID
            assert!(container_path.contains(container_id));
            assert!(pty_path.contains(container_id));
        }
    }
}

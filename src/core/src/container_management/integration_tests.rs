#[cfg(test)]
mod integration_tests {
    use crate::configuration::{Protocol, ServiceConfig};
    use crate::container_management::ContainerManager;
    use crate::error_handling::types::ContainerError;
    use std::time::Duration;
    use tokio::time::timeout;

    fn create_test_service(name: &str, port: u16, protocol: Protocol) -> ServiceConfig {
        ServiceConfig {
            name: name.to_string(),
            port,
            protocol,
            container_image: format!("test-{}", name),
            enabled: true,
            header_patterns: vec![format!("{}-pattern", name)],
            banner_response: Some(format!("{} banner", name)),
        }
    }

    fn is_running_as_root() -> bool {
        std::process::Command::new("id")
            .arg("-u")
            .output()
            .map(|output| {
                if output.status.success() {
                    String::from_utf8_lossy(&output.stdout).trim() == "0"
                } else {
                    false
                }
            })
            .unwrap_or(false)
    }

    fn is_systemd_nspawn_available() -> bool {
        std::process::Command::new("systemd-nspawn")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    #[tokio::test]
    #[ignore = "requires systemd-nspawn and root privileges"]
    async fn test_container_manager_initialization() {
        if !is_systemd_nspawn_available() || !is_running_as_root() {
            return;
        }

        let manager = ContainerManager::new().expect("Failed to create container manager");

        let stats = manager.get_container_stats();
        assert_eq!(stats.active_count, 0);
        assert_eq!(stats.total_created, 0);
        assert_eq!(stats.failed_count, 0);

        let containers = manager.list_active_containers();
        assert!(containers.is_empty());

        println!("ContainerManager initialized successfully");
    }

    #[tokio::test]
    #[ignore = "requires systemd-nspawn and root privileges"]
    async fn test_container_lifecycle_end_to_end() {
        if !is_systemd_nspawn_available() || !is_running_as_root() {
            return;
        }

        let mut manager =
            ContainerManager::new().expect("Failed to create container manager for lifecycle test");

        let http_service = create_test_service("http", 80, Protocol::TCP);

        println!("Creating HTTP service container...");
        let handle = timeout(
            Duration::from_secs(30),
            manager.create_container(&http_service),
        )
        .await
        .expect("Container creation timed out")
        .expect("Failed to create container");

        assert!(handle.id.starts_with("miel-http-"));
        assert_eq!(handle.service_name, "http");
        assert_eq!(handle.port, 80);
        assert!(handle.host_port > 0);
        assert!(handle.host_port != 80);

        println!(
            "Container created: {} on port {}",
            handle.id, handle.host_port
        );

        let stats = manager.get_container_stats();
        assert_eq!(stats.active_count, 1);
        assert_eq!(stats.total_created, 1);
        assert_eq!(stats.failed_count, 0);

        let containers = manager.list_active_containers();
        assert_eq!(containers.len(), 1);
        assert!(containers.contains(&handle.id));

        let retrieved = manager.get_container(&handle.id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().service_name, "http");

        println!("Cleaning up container...");
        manager
            .cleanup_container(handle)
            .await
            .expect("Failed to cleanup container");

        let stats = manager.get_container_stats();
        assert_eq!(stats.active_count, 0);
        assert_eq!(stats.total_created, 1);
        assert_eq!(stats.failed_count, 0);

        let containers = manager.list_active_containers();
        assert!(containers.is_empty());

        println!("Container lifecycle test completed successfully");
    }

    #[tokio::test]
    async fn test_error_conditions() {
        if is_systemd_nspawn_available() && is_running_as_root() {
            eprintln!("Skipping error conditions test: all prerequisites are met");
            return;
        }

        match ContainerManager::new() {
            Ok(_) => panic!("Manager should have failed without prerequisites"),
            Err(ContainerError::RuntimeNotAvailable) => {
                println!("Correctly failed with RuntimeNotAvailable");
            }
            Err(ContainerError::StartFailed(msg)) => {
                assert!(msg.contains("root privileges"));
                println!("Correctly failed with insufficient privileges");
            }
            Err(e) => panic!("Unexpected error type: {:?}", e),
        }
    }
}

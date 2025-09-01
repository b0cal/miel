#[cfg(test)]
mod tests {
    use crate::configuration::{Protocol, ServiceConfig};
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
}

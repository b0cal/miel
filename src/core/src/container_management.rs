//! Container management subsystem.
//!
//! This module exposes a minimal API to create and manage lightweight containers
//! for honeypot services. The current implementation targets `systemd-nspawn`
//! and focuses on simple lifecycle management and bookkeeping.
//!
//! Re-exports:
//! - [`ContainerManager`]: main entry point to create/cleanup containers.
//! - [`ContainerHandle`], [`ContainerStats`], [`Runtime`]: core types.
//!
//! Example (non-running):
//! ```ignore
//! use miel::container_management::{ContainerManager, ContainerStats};
//!
//! // Create a manager (will fail if `systemd-nspawn` is not available)
//! let manager = ContainerManager::new()?;
//! let stats: ContainerStats = manager.get_container_stats();
//! println!("active: {}", stats.active_count);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod container_manager;
#[cfg(test)]
pub mod integration_tests;
#[cfg(test)]
pub mod tests;
pub mod types;

pub use container_manager::ContainerManager;
pub use types::{ContainerHandle, ContainerStats, Runtime};

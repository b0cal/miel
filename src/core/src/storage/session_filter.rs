//! Helpers for building `SessionFilter` values.
//!
//! This module re-exports `SessionFilter` and provides convenience builders
//! for common query criteria.

// Re-export SessionFilter
pub use crate::storage::types::SessionFilter;

/// Build a `SessionFilter` that matches sessions by exact service name.
#[allow(dead_code)]
pub fn by_service_name<S: Into<String>>(name: S) -> SessionFilter {
    SessionFilter { service_name: Some(name.into()), ..Default::default() }
}

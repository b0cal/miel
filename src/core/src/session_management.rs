//! Session management core module.
//!
//! This module provides the core types and submodules for managing user sessions,
//! including session status, active sessions, and session management logic.

use serde::{Deserialize, Serialize};

/// Submodule for handling active session logic.
pub mod active_session;
/// Submodule for session data structures and utilities.
pub mod session;
/// Submodule for session manager implementation.
pub mod session_manager;

/// Represents the current status of a session.
///
/// Variants:
/// - `Pending`: The session is awaiting activation.
/// - `Active`: The session is currently active.
/// - `Completed`: The session has finished successfully.
/// - `Error`: The session encountered an error.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Pending,
    Active,
    Completed,
    Error,
}

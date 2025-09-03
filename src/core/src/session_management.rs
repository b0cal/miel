pub mod session_manager;
pub mod session;
pub mod active_session;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    Pending,
    Active,
    Completed,
    Error,
}
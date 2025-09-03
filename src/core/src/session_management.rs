pub mod active_session;
pub mod session;
pub mod session_manager;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    Pending,
    Active,
    Completed,
    Error,
}

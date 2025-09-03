pub mod active_session;
pub mod session;
pub mod session_manager;

pub enum SessionStatus {
    Pending,
    Active,
    Completed,
    Error,
}

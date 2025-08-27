pub mod session_manager;
pub mod session;
pub mod active_session;

pub enum SessionStatus {
    Pending,
    Active,
    Completed,
    Error
}
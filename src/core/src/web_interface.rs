// Web Interface module root
pub mod responses;
pub mod routes;
mod types;
pub mod web_server;

// Re-export commonly used items
pub use routes::*;
pub use web_server::*;

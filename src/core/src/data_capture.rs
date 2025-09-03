//! Data capture subsystem: TCP proxying, stdio parsing, and persistence.
//!
//! This module groups the building blocks used to record honeypot sessions:
//! - `tcp_capture`: full‑duplex TCP forwarding while recording bytes and timestamps
//! - `stdio_capture`: parse activity logs or snapshot a PTY into stdin/stdout/stderr streams
//! - `storage`: trait to persist/retrieve capture artifacts
//! - `recorder`: high‑level façade that orchestrates the above for one session
//!
//! Re‑exports: see the items below for quick access in downstream code.

pub mod recorder;
pub mod stdio_capture;
pub mod storage;
pub mod tcp_capture;
pub mod types;

pub use recorder::StreamRecorder;
pub use stdio_capture::StdioCapture;
pub use storage::Storage;
pub use tcp_capture::TcpCapture;
pub use types::{CaptureArtifacts, Direction, StdioStream};
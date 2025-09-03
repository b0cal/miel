//! Stream recording orchestration for a single session.
//!
//! This module provides `StreamRecorder`, a small façade that ties together
//! bidirectional TCP proxying (via `TcpCapture`) and best‑effort stdio/PTY
//! snapshotting (via `StdioCapture`) for one honeypot session. It aggregates
//! the resulting byte streams and metadata into `CaptureArtifacts` and persists
//! them through the injected `Storage` implementation.
//!
//! Highlights
//! - Full‑duplex TCP proxy with per‑direction buffering and timestamps
//! - Graceful EOF propagation to avoid hangs (shutdown of the peer writer)
//! - Optional PTY snapshot for stdout/stderr capture
//! - Pluggable persistence through `Storage` (dependency injected)
//! - Rich logging at TRACE/DEBUG/INFO
//!
//! Minimal usage
//! ```no_run
//! use std::sync::Arc;
//! use tokio::net::TcpStream;
//! use uuid::Uuid;
//! use miel::data_capture::StreamRecorder;
//! use miel::data_capture::CaptureArtifacts;
//!
//! // Your Storage implementation just needs to persist/retrieve artifacts.
//! struct MyStorage;
//! impl miel::data_capture::Storage for MyStorage {
//!     fn save_capture_artifacts(&self, _a: &CaptureArtifacts) -> Result<(), miel::error_handling::types::StorageError> { Ok(()) }
//!     fn get_capture_artifacts(&self, _id: Uuid) -> Result<CaptureArtifacts, miel::error_handling::types::StorageError> { todo!() }
//! }
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let storage: Arc<dyn miel::data_capture::Storage> = Arc::new(MyStorage);
//! let recorder = StreamRecorder::new(Uuid::new_v4(), storage);
//!
//! // Example sockets; in real code they come from your listener/container.
//! let (client, server) = (TcpStream::connect("127.0.0.1:1").await?, TcpStream::connect("127.0.0.1:2").await?);
//! let _ = recorder.start_tcp_proxy(client, server).await; // ignore errors in example
//! let _ = recorder.finalize_capture(); // ignore errors in example
//! # Ok(())
//! # }
//! ```

use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use log::{debug, info};
use tokio::net::TcpStream;
use uuid::Uuid;

use crate::error_handling::types::CaptureError;

use super::stdio_capture::StdioCapture;
use super::storage::Storage;
use super::tcp_capture::TcpCapture;
use super::types::CaptureArtifacts;

/// Orchestrates network and stdio capture for a single session.
///
/// A `StreamRecorder` owns per‑session capture state and a reference to
/// a pluggable [`Storage`] implementation. It exposes three operations:
/// - [`start_tcp_proxy`]: full‑duplex forwarding between client and container
///   sockets while recording both directions with timestamps.
/// - [`start_stdio_capture`]: optional, best‑effort snapshot from a PTY handle
///   (e.g. container stdout/stderr); safe to call zero or multiple times.
/// - [`finalize_capture`]: aggregates all data into [`CaptureArtifacts`],
///   persists them via [`Storage`], and returns the artifacts to the caller.
///
/// Logging
/// - DEBUG/INFO for lifecycle milestones
/// - TRACE for short (64‑byte) previews of captured chunks
pub struct StreamRecorder {
    /// Unique session identifier (used to correlate logs and persisted data).
    session_id: Uuid,
    /// TCP capture engine (both directions with timestamps).
    tcp_capture: Arc<TcpCapture>,
    /// Optional stdio/PTY snapshotter for the current session.
    stdio_capture: Option<Arc<StdioCapture>>,
    /// Pluggable persistence backend.
    storage: Arc<dyn Storage>,
    /// Session start wall‑clock time (UTC), used to compute duration.
    start_time: DateTime<Utc>,
}

impl StreamRecorder {
    /// Creates a new `StreamRecorder` for the given `session_id` and `storage`.
    ///
    /// The recorder holds only lightweight buffers and references; it’s cheap to
    /// construct and clone the underlying `Arc` values as needed by your orchestration.
    pub fn new(session_id: Uuid, storage: Arc<dyn Storage>) -> Self {
        debug!("[{}] StreamRecorder created", session_id);
        Self {
            session_id,
            tcp_capture: Arc::new(TcpCapture::new(session_id)),
            stdio_capture: None,
            storage,
            start_time: Utc::now(),
        }
    }

    /// Starts a full‑duplex TCP proxy between the `client_stream` and the
    /// `container_stream`, recording both directions.
    ///
    /// Behavior
    /// - Forwards client→container and container→client using owned split halves.
    /// - On EOF from one side, gracefully shuts down the opposite writer to wake
    ///   the peer task and terminate without hangs.
    /// - Records bytes and timestamps for both directions.
    ///
    /// Errors
    /// - Returns [`CaptureError::TcpStreamError`] for read/write failures.
    pub async fn start_tcp_proxy(
        &self,
        client_stream: TcpStream,
        container_stream: TcpStream,
    ) -> Result<(), CaptureError> {
        Arc::clone(&self.tcp_capture)
            .proxy_and_record(client_stream, container_stream)
            .await
    }

    /// Take a best‑effort PTY snapshot for stdio capture (non‑blocking where
    /// possible) and appends results internally.
    ///
    /// Notes
    /// - Safe to call multiple times; each call attempts a short read.
    /// - It’s OK if the PTY isn’t readable yet (WouldBlock is ignored).
    ///
    /// Errors
    /// - Returns [`CaptureError::StdioError`] for non‑recoverable IO failures.
    pub fn start_stdio_capture(&mut self, pty_master: std::fs::File) -> Result<(), CaptureError> {
        debug!("[{}] Starting stdio capture snapshot", self.session_id);
        let cap = self
            .stdio_capture
            .get_or_insert_with(|| Arc::new(StdioCapture::new(self.session_id)))
            .clone();
        cap.capture_pty(pty_master)
    }

    /// Parse a unified activity log file and append its STDIN/STDOUT/STDERR
    /// content to this recorder's stdio buffers.
    pub fn parse_stdio_log_from_file<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<(), CaptureError> {
        let cap = self
            .stdio_capture
            .get_or_insert_with(|| Arc::new(StdioCapture::new(self.session_id)))
            .clone();
        cap.as_ref().capture_activity_log_from_path(path)
    }

    /// Aggregates TCP and stdio buffers into [`CaptureArtifacts`], computes
    /// totals and duration, persists them via [`Storage`], and returns the
    /// artifacts to the caller.
    ///
    /// Returns
    /// - Persisted [`CaptureArtifacts`] for this session.
    ///
    /// Errors
    /// - Returns [`CaptureError::StorageError`] if the storage backend fails to persist.
    pub fn finalize_capture(&self) -> Result<CaptureArtifacts, CaptureError> {
        let (c2s, s2c, tcp_ts) = self.tcp_capture.get_artifacts();

        let (stdin, stdout, stderr, stdio_ts) = if let Some(ref stdio) = self.stdio_capture {
            stdio.get_artifacts()
        } else {
            (Vec::new(), Vec::new(), Vec::new(), Vec::new())
        };

        let total_bytes: u64 =
            (c2s.len() + s2c.len() + stdin.len() + stdout.len() + stderr.len()) as u64;
        let duration = Utc::now() - self.start_time;

        info!(
            "[{}] Finalized capture: total_bytes={}, tcp_c2s={}, tcp_s2c={}, stdout={}, stderr={}, duration={:?}",
            self.session_id,
            total_bytes,
            c2s.len(),
            s2c.len(),
            stdout.len(),
            stderr.len(),
            duration
        );

        let artifacts = CaptureArtifacts {
            session_id: self.session_id,
            tcp_client_to_container: c2s,
            tcp_container_to_client: s2c,
            stdio_stdin: stdin,
            stdio_stdout: stdout,
            stdio_stderr: stderr,
            tcp_timestamps: tcp_ts,
            stdio_timestamps: stdio_ts,
            total_bytes,
            duration,
        };

        self.storage
            .save_capture_artifacts(&artifacts)
            .map_err(CaptureError::StorageError)?;

        Ok(artifacts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::Mutex as StdMutex;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use crate::data_capture::storage::Storage;
    use crate::data_capture::types::Direction;
    use crate::error_handling::types::StorageError;

    struct MemStorage {
        inner: StdMutex<Option<CaptureArtifacts>>,
    }

    impl MemStorage {
        fn new() -> Self {
            Self {
                inner: StdMutex::new(None),
            }
        }
    }

    impl Storage for MemStorage {
        fn save_capture_artifacts(&self, artifacts: &CaptureArtifacts) -> Result<(), StorageError> {
            *self.inner.lock().unwrap() = Some(artifacts.clone());
            Ok(())
        }

        fn get_capture_artifacts(
            &self,
            _session_id: Uuid,
        ) -> Result<CaptureArtifacts, StorageError> {
            self.inner
                .lock()
                .unwrap()
                .clone()
                .ok_or(StorageError::ReadFailed)
        }
    }

    async fn tcp_pair() -> std::io::Result<(TcpStream, TcpStream)> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;
        let addr = listener.local_addr()?;

        let client = tokio::spawn(async move { TcpStream::connect(addr).await });
        let (server_side, _) = listener.accept().await?;
        let client = client.await.unwrap()?;
        Ok((server_side, client))
    }

    #[tokio::test]
    async fn tcp_proxy_captures_data() {
        let _ = env_logger::builder().is_test(true).try_init();
        let (client_server_side, mut client_outside) = tcp_pair().await.unwrap();
        let (container_server_side, mut container_inside) = tcp_pair().await.unwrap();

        let storage: Arc<dyn Storage> = Arc::new(MemStorage::new());
        let recorder = Arc::new(StreamRecorder::new(Uuid::new_v4(), storage));

        let rec2 = Arc::clone(&recorder);
        let proxy = tokio::spawn(async move {
            rec2.start_tcp_proxy(client_server_side, container_server_side)
                .await
        });

        // Send payload client -> container
        client_outside
            .write_all(b"hello")
            .await
            .expect("write client->container");
        client_outside.flush().await.ok();

        let mut buf = [0u8; 16];
        let n = container_inside
            .read(&mut buf)
            .await
            .expect("read in container");
        assert_eq!(&buf[..n], b"hello");

        // Send response container -> client
        container_inside
            .write_all(b"pong")
            .await
            .expect("write container->client");
        container_inside.flush().await.ok();

        let mut rbuf = [0u8; 16];
        let rn = client_outside
            .read(&mut rbuf)
            .await
            .expect("read at client");
        assert_eq!(&rbuf[..rn], b"pong");

        // Gracefully shutdown both write halves to trigger EOF on proxy tasks
        client_outside.shutdown().await.ok();
        container_inside.shutdown().await.ok();

        // Close streams to end proxy
        drop(container_inside);
        drop(client_outside);

        // Wait for proxy completion with timeout to avoid hangs in CI
        let res = tokio::time::timeout(std::time::Duration::from_secs(2), proxy).await;
        match res {
            Ok(join_res) => {
                join_res.expect("proxy join").expect("proxy ok");
            }
            Err(_) => panic!("proxy task timed out"),
        }

        let artifacts = recorder.finalize_capture().expect("finalize ok");
        assert!(artifacts.total_bytes >= 5);
        assert!(artifacts
            .tcp_timestamps
            .iter()
            .any(|(_, dir, n)| *dir == Direction::ClientToContainer && *n > 0));
        assert!(artifacts
            .tcp_timestamps
            .iter()
            .any(|(_, dir, n)| *dir == Direction::ContainerToClient && *n > 0));
    }
}

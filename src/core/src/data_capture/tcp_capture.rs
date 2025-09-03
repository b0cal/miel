//! TCP capture: full‑duplex proxying while recording traffic and timestamps.
//!
//! This module exposes [`TcpCapture`], which forwards data between a client and
//! a container/service while buffering both directions and recording per‑chunk
//! timestamps. It is used by the higher‑level `StreamRecorder` façade.

use std::io;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use log::trace;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::task::JoinSet;
use uuid::Uuid;

use super::types::Direction;
use crate::error_handling::types::CaptureError;

type TcpTimestamps = Vec<(DateTime<Utc>, Direction, usize)>;
type TcpArtifacts = (Vec<u8>, Vec<u8>, TcpTimestamps);

#[derive(Debug)]
/// Records TCP traffic for one session while acting as a transparent proxy.
pub struct TcpCapture {
    pub(crate) session_id: Uuid,
    pub(crate) client_to_container: Mutex<Vec<u8>>,
    pub(crate) container_to_client: Mutex<Vec<u8>>,
    pub(crate) timestamps: Mutex<TcpTimestamps>,
}

impl TcpCapture {
    /// Create a new `TcpCapture` instance for `session_id`.
    pub fn new(session_id: Uuid) -> Self {
        Self {
            session_id,
            client_to_container: Mutex::new(Vec::new()),
            container_to_client: Mutex::new(Vec::new()),
            timestamps: Mutex::new(Vec::new()),
        }
    }

    /// Forward data in both directions while recording bytes and timestamps.
    ///
    /// Behavior
    /// - Spawns two tasks (client→container and container→client).
    /// - Gracefully propagates EOF by shutting down the opposite writer.
    /// - Buffers payloads and pushes `(timestamp, direction, len)` entries.
    ///
    /// Errors
    /// - Returns [`CaptureError::TcpStreamError`] for I/O or task failures.
    pub async fn proxy_and_record(
        self: Arc<Self>,
        client_stream: TcpStream,
        container_stream: TcpStream,
    ) -> Result<(), CaptureError> {
        let (cr, cw) = client_stream.into_split();
        let (sr, sw) = container_stream.into_split();

        trace!("[{:?}] starting tcp proxy", self.session_id);

        let mut set = JoinSet::new();

        // Client -> Container (read from client, write to container)
        {
            let this = Arc::clone(&self);
            set.spawn(async move {
                trace!("[{:?}] C->S task started", this.session_id);
                let mut cr = cr;
                let mut sw = sw; // forward to container writer
                let mut buf = vec![0u8; 16 * 1024];
                loop {
                    let n = match cr.read(&mut buf).await {
                        Ok(n) => n,
                        Err(e) => break Err(CaptureError::TcpStreamError(e)),
                    };
                    if n == 0 {
                        trace!(
                            "[{:?}] C->S EOF; shutting down server writer",
                            this.session_id
                        );
                        let _ = sw.shutdown().await; // signal EOF to container side
                        break Ok(());
                    }
                    if let Err(e) = sw.write_all(&buf[..n]).await {
                        break Err(CaptureError::TcpStreamError(e));
                    }
                    // record and trace
                    {
                        let mut data = this.client_to_container.lock().unwrap();
                        data.extend_from_slice(&buf[..n]);
                    }
                    {
                        let mut ts = this.timestamps.lock().unwrap();
                        ts.push((Utc::now(), Direction::ClientToContainer, n));
                    }
                    let preview = &buf[..std::cmp::min(n, 64)];
                    trace!(
                        "[{:?}] captured C->S {} bytes: {}{}",
                        this.session_id,
                        n,
                        String::from_utf8_lossy(preview),
                        if n > 64 { " ..." } else { "" }
                    );
                }
            });
        }

        // Container -> Client (read from container, write to client)
        {
            let this = Arc::clone(&self);
            set.spawn(async move {
                trace!("[{:?}] S->C task started", this.session_id);
                let mut sr = sr;
                let mut cw = cw; // forward to client writer
                let mut buf = vec![0u8; 16 * 1024];
                loop {
                    let n = match sr.read(&mut buf).await {
                        Ok(n) => n,
                        Err(e) => break Err(CaptureError::TcpStreamError(e)),
                    };
                    if n == 0 {
                        trace!(
                            "[{:?}] S->C EOF; shutting down client writer",
                            this.session_id
                        );
                        let _ = cw.shutdown().await; // signal EOF to client side
                        break Ok(());
                    }
                    if let Err(e) = cw.write_all(&buf[..n]).await {
                        break Err(CaptureError::TcpStreamError(e));
                    }
                    // record and trace
                    {
                        let mut data = this.container_to_client.lock().unwrap();
                        data.extend_from_slice(&buf[..n]);
                    }
                    {
                        let mut ts = this.timestamps.lock().unwrap();
                        ts.push((Utc::now(), Direction::ContainerToClient, n));
                    }
                    let preview = &buf[..std::cmp::min(n, 64)];
                    trace!(
                        "[{:?}] captured S->C {} bytes: {}{}",
                        this.session_id,
                        n,
                        String::from_utf8_lossy(preview),
                        if n > 64 { " ..." } else { "" }
                    );
                }
            });
        }

        while let Some(res) = set.join_next().await {
            res.map_err(|e| CaptureError::TcpStreamError(io::Error::other(e)))??;
        }

        trace!("[{:?}] tcp proxy completed", self.session_id);
        Ok(())
    }

    /// Return copies of client→container, container→client, and timestamp log.
    pub fn get_artifacts(&self) -> TcpArtifacts {
        let a = self.client_to_container.lock().unwrap().clone();
        let b = self.container_to_client.lock().unwrap().clone();
        let t = self.timestamps.lock().unwrap().clone();
        (a, b, t)
    }
}

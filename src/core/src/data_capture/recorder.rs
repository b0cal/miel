use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::net::TcpStream;
use uuid::Uuid;

use crate::error_handling::types::{CaptureError, StorageError};

use super::storage::Storage;
use super::tcp_capture::TcpCapture;
use super::types::CaptureArtifacts;
use super::stdio_capture::StdioCapture;

pub struct StreamRecorder {
    session_id: Uuid,
    tcp_capture: Arc<TcpCapture>,
    stdio_capture: Option<StdioCapture>,
    storage: Arc<dyn Storage>,
    start_time: DateTime<Utc>,
}

impl StreamRecorder {
    pub fn new(session_id: Uuid, storage: Arc<dyn Storage>) -> Self {
        Self {
            session_id,
            tcp_capture: Arc::new(TcpCapture::new(session_id)),
            stdio_capture: None,
            storage,
            start_time: Utc::now(),
        }
    }

    pub async fn start_tcp_proxy(
        &self,
        client_stream: TcpStream,
        container_stream: TcpStream,
    ) -> Result<(), CaptureError> {
        Arc::clone(&self.tcp_capture)
            .proxy_and_record(client_stream, container_stream)
            .await
    }

    pub fn start_stdio_capture(&mut self, pty_master: std::fs::File) -> Result<(), CaptureError> {
        let cap = self
            .stdio_capture
            .get_or_insert_with(|| StdioCapture::new(self.session_id));
        cap.capture_pty(pty_master)
    }

    pub fn finalize_capture(&self) -> Result<CaptureArtifacts, CaptureError> {
        let (c2s, s2c, tcp_ts) = self.tcp_capture.get_artifacts();

        let (stdin, stdout, stderr, stdio_ts) = if let Some(ref stdio) = self.stdio_capture {
            stdio.get_artifacts()
        } else {
            (Vec::new(), Vec::new(), Vec::new(), Vec::new())
        };

        let total_bytes: u64 = (c2s.len() + s2c.len() + stdin.len() + stdout.len() + stderr.len())
            as u64;
        let duration = Utc::now() - self.start_time;

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
    use std::sync::Mutex as StdMutex;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use std::sync::Arc;

    use crate::data_capture::storage::Storage;
    use crate::data_capture::types::Direction;

    struct MemStorage {
        inner: StdMutex<Option<CaptureArtifacts>>,
    }

    impl MemStorage {
        fn new() -> Self {
            Self { inner: StdMutex::new(None) }
        }
    }

    impl Storage for MemStorage {
        fn save_capture_artifacts(
            &self,
            artifacts: &CaptureArtifacts,
        ) -> Result<(), StorageError> {
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
            rec2
                .start_tcp_proxy(client_server_side, container_server_side)
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
        use tokio::io::AsyncWriteExt as _;
        client_outside.shutdown().await.ok();
        container_inside.shutdown().await.ok();

        // Close streams to end proxy
        drop(container_inside);
        drop(client_outside);

        // Wait proxy completion with timeout to avoid hangs in CI
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

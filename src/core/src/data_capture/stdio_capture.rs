use std::io::{self, Read};
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error_handling::types::CaptureError;
use super::types::StdioStream;

#[derive(Debug)]
pub struct StdioCapture {
    pub(crate) session_id: Uuid,
    pub(crate) stdin_data: Mutex<Vec<u8>>,
    pub(crate) stdout_data: Mutex<Vec<u8>>,
    pub(crate) stderr_data: Mutex<Vec<u8>>,
    pub(crate) timestamps: Mutex<Vec<(DateTime<Utc>, StdioStream, usize)>>,
}

impl StdioCapture {
    pub fn new(session_id: Uuid) -> Self {
        Self {
            session_id,
            stdin_data: Mutex::new(Vec::new()),
            stdout_data: Mutex::new(Vec::new()),
            stderr_data: Mutex::new(Vec::new()),
            timestamps: Mutex::new(Vec::new()),
        }
    }

    pub fn capture_pty(&self, mut pty_master: std::fs::File) -> Result<(), CaptureError> {
        let mut buf = [0u8; 4096];
        match pty_master.read(&mut buf) {
            Ok(0) => {}
            Ok(n) => {
                self.stdout_data.lock().unwrap().extend_from_slice(&buf[..n]);
                self.timestamps
                    .lock()
                    .unwrap()
                    .push((Utc::now(), StdioStream::Stdout, n));
            }
            Err(e) => {
                if e.kind() != io::ErrorKind::WouldBlock {
                    return Err(CaptureError::StdioError(e));
                }
            }
        }
        Ok(())
    }

    pub fn get_artifacts(
        &self,
    ) -> (
        Vec<u8>,
        Vec<u8>,
        Vec<u8>,
        Vec<(DateTime<Utc>, StdioStream, usize)>,
    ) {
        let i = self.stdin_data.lock().unwrap().clone();
        let o = self.stdout_data.lock().unwrap().clone();
        let e = self.stderr_data.lock().unwrap().clone();
        let t = self.timestamps.lock().unwrap().clone();
        (i, o, e, t)
    }
}


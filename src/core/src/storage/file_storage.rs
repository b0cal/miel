use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use log::{debug, error, info};
use uuid::Uuid;
use crate::error_handling::types::StorageError;
use crate::storage::storage::Storage;
use crate::storage::types::{CaptureArtifacts, Direction, Session, SessionFilter, StdioStream};

pub struct FileStorage {
    base_path: PathBuf,
    session_index: Mutex<HashMap<Uuid, PathBuf>>, // maps id to session file path
    artifacts_path: PathBuf,
}

impl FileStorage {
    pub fn new<P: AsRef<Path>>(base_path: P) -> Result<Self, StorageError> {
        let base_path = base_path.as_ref().to_path_buf();
        let sessions_dir = base_path.join("sessions");
        let interactions_dir = base_path.join("interactions");
        let artifacts_path = base_path.join("artifacts");

        fs::create_dir_all(&sessions_dir).map_err(|e| { error!("Failed to create sessions dir {}: {}", sessions_dir.display(), e); StorageError::WriteFailed })?;
        fs::create_dir_all(&interactions_dir).map_err(|e| { error!("Failed to create interactions dir {}: {}", interactions_dir.display(), e); StorageError::WriteFailed })?;
        fs::create_dir_all(&artifacts_path).map_err(|e| { error!("Failed to create artifacts dir {}: {}", artifacts_path.display(), e); StorageError::WriteFailed })?;
        info!("FileStorage initialized at {}", base_path.display());

        Ok(Self {
            base_path,
            session_index: Mutex::new(HashMap::new()),
            artifacts_path,
        })
    }

    /// Construct FileStorage using env var MIEL_FILE_STORAGE_DIR if set, otherwise current directory.
    pub fn new_default() -> Result<Self, StorageError> {
        if let Ok(dir) = std::env::var("MIEL_FILE_STORAGE_DIR") {
            info!("Using FileStorage from MIEL_FILE_STORAGE_DIR: {}", dir);
            return Self::new(PathBuf::from(dir));
        }
        let cwd = std::env::current_dir().map_err(|e| { error!("Failed to get current dir: {}", e); StorageError::ReadFailed })?;
        info!("Using FileStorage at current directory: {}", cwd.display());
        Self::new(cwd)
    }

    fn sessions_dir(&self) -> PathBuf { self.base_path.join("sessions") }
    fn interactions_dir(&self) -> PathBuf { self.base_path.join("interactions") }
    fn artifacts_dir_for(&self, id: Uuid) -> PathBuf { self.artifacts_path.join(id.to_string()) }

    fn session_file_path(&self, id: Uuid) -> PathBuf { self.sessions_dir().join(format!("{}.session", id)) }

    fn write_session_file(&self, session: &Session) -> Result<(), StorageError> {
        let path = self.session_file_path(session.id);
        let mut f = File::create(&path).map_err(|e| { error!("Failed to create session file {}: {}", path.display(), e); StorageError::WriteFailed })?;
        // Simple KV text format
        writeln!(f, "id: {}", session.id).map_err(|e| { error!("Failed to write session file {}: {}", path.display(), e); StorageError::WriteFailed })?;
        writeln!(f, "service_name: {}", session.service_name).map_err(|e| { error!("Failed to write session file {}: {}", path.display(), e); StorageError::WriteFailed })?;
        writeln!(f, "client_addr: {}", session.client_addr).map_err(|e| { error!("Failed to write session file {}: {}", path.display(), e); StorageError::WriteFailed })?;
        writeln!(f, "start_time: {}", session.start_time.to_rfc3339()).map_err(|e| { error!("Failed to write session file {}: {}", path.display(), e); StorageError::WriteFailed })?;
        if let Some(end) = session.end_time { writeln!(f, "end_time: {}", end.to_rfc3339()).map_err(|e| { error!("Failed to write session file {}: {}", path.display(), e); StorageError::WriteFailed })?; } else { writeln!(f, "end_time: none").map_err(|e| { error!("Failed to write session file {}: {}", path.display(), e); StorageError::WriteFailed })?; }
        if let Some(ref cid) = session.container_id { writeln!(f, "container_id: {}", cid).map_err(|e| { error!("Failed to write session file {}: {}", path.display(), e); StorageError::WriteFailed })?; } else { writeln!(f, "container_id: none").map_err(|e| { error!("Failed to write session file {}: {}", path.display(), e); StorageError::WriteFailed })?; }
        writeln!(f, "bytes_transferred: {}", session.bytes_transferred).map_err(|e| { error!("Failed to write session file {}: {}", path.display(), e); StorageError::WriteFailed })?;
        let status_str = match session.status { crate::session_management::SessionStatus::Pending => "Pending", crate::session_management::SessionStatus::Active => "Active", crate::session_management::SessionStatus::Completed => "Completed", crate::session_management::SessionStatus::Error => "Error" };
        writeln!(f, "status: {}", status_str).map_err(|e| { error!("Failed to write session file {}: {}", path.display(), e); StorageError::WriteFailed })?;

        // update index
        if let Ok(mut idx) = self.session_index.lock() { idx.insert(session.id, path.clone()); }
        info!("Saved session {} to {}", session.id, path.display());
        Ok(())
    }

    fn parse_session_file(&self, path: &Path) -> Result<Session, StorageError> {
        let mut content = String::new();
        File::open(path).and_then(|mut f| f.read_to_string(&mut content)).map_err(|e| { error!("Failed to read session file {}: {}", path.display(), e); StorageError::ReadFailed })?;
        let mut map: HashMap<String, String> = HashMap::new();
        for line in content.lines() {
            if let Some((k, v)) = line.split_once(": ") { map.insert(k.trim().to_string(), v.trim().to_string()); }
        }
        let id = map.get("id").ok_or_else(|| { error!("Missing id in session file {}", path.display()); StorageError::ReadFailed })?.parse::<Uuid>().map_err(|e| { error!("Invalid UUID in {}: {}", path.display(), e); StorageError::ReadFailed })?;
        let service_name = map.remove("service_name").ok_or_else(|| { error!("Missing service_name in {}", path.display()); StorageError::ReadFailed })?;
        let client_addr_str = map.remove("client_addr").ok_or_else(|| { error!("Missing client_addr in {}", path.display()); StorageError::ReadFailed })?;
        let client_addr = client_addr_str.parse().map_err(|e| { error!("Invalid client_addr in {}: {}", path.display(), e); StorageError::ReadFailed })?;
        let start_time = map.remove("start_time").ok_or_else(|| { error!("Missing start_time in {}", path.display()); StorageError::ReadFailed }).and_then(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)).map_err(|e| { error!("Invalid start_time in {}: {}", path.display(), e); StorageError::ReadFailed }))?;
        let end_time = map.remove("end_time").and_then(|s| if s == "none" { None } else { Some(s) }).map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)).map_err(|e| { error!("Invalid end_time in {}: {}", path.display(), e); StorageError::ReadFailed })).transpose()?;
        let container_id = map.remove("container_id").and_then(|s| if s == "none" { None } else { Some(s) });
        let bytes_transferred = map.remove("bytes_transferred").ok_or_else(|| { error!("Missing bytes_transferred in {}", path.display()); StorageError::ReadFailed })?.parse::<u64>().map_err(|e| { error!("Invalid bytes_transferred in {}: {}", path.display(), e); StorageError::ReadFailed })?;
        let status = match map.remove("status").ok_or_else(|| { error!("Missing status in {}", path.display()); StorageError::ReadFailed })?.as_str() {
            "Pending" => crate::session_management::SessionStatus::Pending,
            "Active" => crate::session_management::SessionStatus::Active,
            "Completed" => crate::session_management::SessionStatus::Completed,
            _ => crate::session_management::SessionStatus::Error,
        };
        debug!("Parsed session {} from {}", id, path.display());
        Ok(Session { id, service_name, client_addr, start_time, end_time, container_id, bytes_transferred, status })
    }
}

impl Storage for FileStorage {
    fn save_session(&self, session: &Session) -> Result<(), StorageError> {
        self.write_session_file(session)
    }

    fn get_sessions(&self, filter: Option<SessionFilter>) -> Result<Vec<Session>, StorageError> {
        let mut sessions = Vec::new();
        for entry in fs::read_dir(self.sessions_dir()).map_err(|e| { error!("Failed to read sessions dir {}: {}", self.sessions_dir().display(), e); StorageError::ReadFailed })? {
            let entry = entry.map_err(|e| { error!("Dir entry error: {}", e); StorageError::ReadFailed })?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("session") {
                if let Ok(sess) = self.parse_session_file(&path) { sessions.push(sess); }
            }
        }
        let original_len = sessions.len();
        if let Some(f) = filter {
            sessions.retain(|s| {
                // service_name
                if let Some(ref name) = f.service_name { if &s.service_name != name { return false; } }
                // time range
                if let Some(start) = f.start_date { if s.start_time < start { return false; } }
                if let Some(end) = f.end_date { if s.end_time.unwrap_or(s.start_time) > end { return false; } }
                // client ip
                if let Some(ip) = f.client_addr { if s.client_addr.ip() != ip { return false; } }
                // status
                if let Some(ref st) = f.status { if &s.status != st { return false; } }
                true
            });
        }
        debug!("Loaded {} session(s), {} after filter", original_len, sessions.len());
        Ok(sessions)
    }

    fn save_interaction(&self, session_id: Uuid, data: &[u8]) -> Result<(), StorageError> {
        let path = self.interactions_dir().join(format!("{}.bin", session_id));
        let mut f = OpenOptions::new().create(true).append(true).open(&path).map_err(|e| { error!("Open append failed {}: {}", path.display(), e); StorageError::WriteFailed })?;
        f.write_all(data).map_err(|e| { error!("Write failed {}: {}", path.display(), e); StorageError::WriteFailed })?;
        debug!("Appended {} byte(s) to {}", data.len(), path.display());
        Ok(())
    }

    fn get_session_data(&self, session_id: Uuid) -> Result<Vec<u8>, StorageError> {
        let path = self.interactions_dir().join(format!("{}.bin", session_id));
        let mut buf = Vec::new();
        File::open(&path).and_then(|mut f| f.read_to_end(&mut buf)).map_err(|e| { error!("Read failed {}: {}", path.display(), e); StorageError::ReadFailed })?;
        debug!("Read {} byte(s) from {}", buf.len(), path.display());
        Ok(buf)
    }

    fn cleanup_old_sessions(&self, older_than: DateTime<Utc>) -> Result<usize, StorageError> {
        let mut removed = 0usize;
        for entry in fs::read_dir(self.sessions_dir()).map_err(|e| { error!("Failed to read sessions dir {}: {}", self.sessions_dir().display(), e); StorageError::ReadFailed })? {
            let entry = entry.map_err(|e| { error!("Dir entry error: {}", e); StorageError::ReadFailed })?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("session") { continue; }
            if let Ok(sess) = self.parse_session_file(&path) {
                let ts = sess.end_time.unwrap_or(sess.start_time);
                if ts < older_than {
                    // remove session file
                    let _ = fs::remove_file(&path);
                    // remove interaction
                    let _ = fs::remove_file(self.interactions_dir().join(format!("{}.bin", sess.id)));
                    // remove artifacts dir
                    let _ = fs::remove_dir_all(self.artifacts_dir_for(sess.id));
                    removed += 1;
                }
            }
        }
        info!("Removed {} old session(s) (cutoff: {})", removed, older_than.to_rfc3339());
        Ok(removed)
    }

    fn save_capture_artifacts(&self, artifacts: &CaptureArtifacts) -> Result<(), StorageError> {
        let dir = self.artifacts_dir_for(artifacts.session_id);
        fs::create_dir_all(&dir).map_err(|e| { error!("Failed to create artifacts dir {}: {}", dir.display(), e); StorageError::WriteFailed })?;
        // tcp dirs
        let mut f = File::create(dir.join("tcp_client_to_container.bin")).map_err(|e| { error!("Create failed: {}: {}", dir.join("tcp_client_to_container.bin").display(), e); StorageError::WriteFailed })?;
        f.write_all(&artifacts.tcp_client_to_container).map_err(|e| { error!("Write failed: {}: {}", dir.join("tcp_client_to_container.bin").display(), e); StorageError::WriteFailed })?;
        let mut f = File::create(dir.join("tcp_container_to_client.bin")).map_err(|e| { error!("Create failed: {}: {}", dir.join("tcp_container_to_client.bin").display(), e); StorageError::WriteFailed })?;
        f.write_all(&artifacts.tcp_container_to_client).map_err(|e| { error!("Write failed: {}: {}", dir.join("tcp_container_to_client.bin").display(), e); StorageError::WriteFailed })?;
        // stdio
        let mut f = File::create(dir.join("stdio_stdin.bin")).map_err(|e| { error!("Create failed: {}: {}", dir.join("stdio_stdin.bin").display(), e); StorageError::WriteFailed })?;
        f.write_all(&artifacts.stdio_stdin).map_err(|e| { error!("Write failed: {}: {}", dir.join("stdio_stdin.bin").display(), e); StorageError::WriteFailed })?;
        let mut f = File::create(dir.join("stdio_stdout.bin")).map_err(|e| { error!("Create failed: {}: {}", dir.join("stdio_stdout.bin").display(), e); StorageError::WriteFailed })?;
        f.write_all(&artifacts.stdio_stdout).map_err(|e| { error!("Write failed: {}: {}", dir.join("stdio_stdout.bin").display(), e); StorageError::WriteFailed })?;
        let mut f = File::create(dir.join("stdio_stderr.bin")).map_err(|e| { error!("Create failed: {}: {}", dir.join("stdio_stderr.bin").display(), e); StorageError::WriteFailed })?;
        f.write_all(&artifacts.stdio_stderr).map_err(|e| { error!("Write failed: {}: {}", dir.join("stdio_stderr.bin").display(), e); StorageError::WriteFailed })?;
        // timestamps CSV-like
        let mut f = File::create(dir.join("tcp_timestamps.csv")).map_err(|e| { error!("Create failed: {}: {}", dir.join("tcp_timestamps.csv").display(), e); StorageError::WriteFailed })?;
        for (ts, dirn, sz) in &artifacts.tcp_timestamps {
            let d = match dirn { Direction::ClientToContainer => "C2S", Direction::ContainerToClient => "S2C" };
            writeln!(f, "{}, {}, {}", ts.to_rfc3339(), d, sz).map_err(|e| { error!("Write failed: {}: {}", dir.join("tcp_timestamps.csv").display(), e); StorageError::WriteFailed })?;
        }
        let mut f = File::create(dir.join("stdio_timestamps.csv")).map_err(|e| { error!("Create failed: {}: {}", dir.join("stdio_timestamps.csv").display(), e); StorageError::WriteFailed })?;
        for (ts, stream, sz) in &artifacts.stdio_timestamps {
            let s = match stream { StdioStream::Stdin => "Stdin", StdioStream::Stdout => "Stdout", StdioStream::Stderr => "Stderr" };
            writeln!(f, "{}, {}, {}", ts.to_rfc3339(), s, sz).map_err(|e| { error!("Write failed: {}: {}", dir.join("stdio_timestamps.csv").display(), e); StorageError::WriteFailed })?;
        }
        // meta
        let mut f = File::create(dir.join("meta.txt")).map_err(|e| { error!("Create failed: {}: {}", dir.join("meta.txt").display(), e); StorageError::WriteFailed })?;
        writeln!(f, "session_id: {}", artifacts.session_id).map_err(|e| { error!("Write failed: {}: {}", dir.join("meta.txt").display(), e); StorageError::WriteFailed })?;
        writeln!(f, "total_bytes: {}", artifacts.total_bytes).map_err(|e| { error!("Write failed: {}: {}", dir.join("meta.txt").display(), e); StorageError::WriteFailed })?;
        // store duration as seconds
        writeln!(f, "duration_secs: {}", artifacts.duration.num_seconds()).map_err(|e| { error!("Write failed: {}: {}", dir.join("meta.txt").display(), e); StorageError::WriteFailed })?;
        info!("Saved artifacts for session {} in {}", artifacts.session_id, dir.display());
        Ok(())
    }

    fn get_capture_artifacts(&self, session_id: Uuid) -> Result<CaptureArtifacts, StorageError> {
        let dir = self.artifacts_dir_for(session_id);
        let read_bin = |name: &str| -> Result<Vec<u8>, StorageError> {
            let p = dir.join(name);
            let mut buf = Vec::new();
            File::open(&p).and_then(|mut f| f.read_to_end(&mut buf)).map_err(|e| { error!("Read failed {}: {}", p.display(), e); StorageError::ReadFailed })?;
            Ok(buf)
        };
        let tcp_client_to_container = read_bin("tcp_client_to_container.bin")?;
        let tcp_container_to_client = read_bin("tcp_container_to_client.bin")?;
        let stdio_stdin = read_bin("stdio_stdin.bin")?;
        let stdio_stdout = read_bin("stdio_stdout.bin")?;
        let stdio_stderr = read_bin("stdio_stderr.bin")?;

        // parse timestamps
        let mut tcp_timestamps: Vec<(DateTime<Utc>, Direction, usize)> = Vec::new();
        let mut s = String::new();
        File::open(dir.join("tcp_timestamps.csv")).and_then(|mut f| f.read_to_string(&mut s)).map_err(|e| { error!("Read failed {}: {}", dir.join("tcp_timestamps.csv").display(), e); StorageError::ReadFailed })?;
        for line in s.lines() {
            let parts: Vec<_> = line.split(',').map(|x| x.trim()).collect();
            if parts.len() != 3 { continue; }
            let ts = DateTime::parse_from_rfc3339(parts[0]).map_err(|e| { error!("Invalid timestamp in tcp_timestamps.csv: {}", e); StorageError::ReadFailed })?.with_timezone(&Utc);
            let dirn = match parts[1] { "C2S" => Direction::ClientToContainer, "S2C" => Direction::ContainerToClient, _ => Direction::ClientToContainer };
            let sz = parts[2].parse::<usize>().map_err(|e| { error!("Invalid size in tcp_timestamps.csv: {}", e); StorageError::ReadFailed })?;
            tcp_timestamps.push((ts, dirn, sz));
        }

        let mut stdio_timestamps: Vec<(DateTime<Utc>, StdioStream, usize)> = Vec::new();
        s.clear();
        File::open(dir.join("stdio_timestamps.csv")).and_then(|mut f| f.read_to_string(&mut s)).map_err(|e| { error!("Read failed {}: {}", dir.join("stdio_timestamps.csv").display(), e); StorageError::ReadFailed })?;
        for line in s.lines() {
            let parts: Vec<_> = line.split(',').map(|x| x.trim()).collect();
            if parts.len() != 3 { continue; }
            let ts = DateTime::parse_from_rfc3339(parts[0]).map_err(|e| { error!("Invalid timestamp in stdio_timestamps.csv: {}", e); StorageError::ReadFailed })?.with_timezone(&Utc);
            let stream = match parts[1] { "Stdin" => StdioStream::Stdin, "Stdout" => StdioStream::Stdout, _ => StdioStream::Stderr };
            let sz = parts[2].parse::<usize>().map_err(|e| { error!("Invalid size in stdio_timestamps.csv: {}", e); StorageError::ReadFailed })?;
            stdio_timestamps.push((ts, stream, sz));
        }

        // meta
        s.clear();
        File::open(dir.join("meta.txt")).and_then(|mut f| f.read_to_string(&mut s)).map_err(|e| { error!("Read failed {}: {}", dir.join("meta.txt").display(), e); StorageError::ReadFailed })?;
        let mut total_bytes = 0u64;
        let mut duration_secs = 0i64;
        for line in s.lines() {
            if let Some((k, v)) = line.split_once(':') {
                let k = k.trim();
                let v = v.trim();
                match k {
                    "total_bytes" => total_bytes = v.parse().unwrap_or(0),
                    "duration_secs" => duration_secs = v.parse().unwrap_or(0),
                    _ => {}
                }
            }
        }
        let duration = chrono::Duration::seconds(duration_secs);
        info!("Loaded artifacts for session {} from {}", session_id, dir.display());

        Ok(CaptureArtifacts { session_id, tcp_client_to_container, tcp_container_to_client, stdio_stdin, stdio_stdout, stdio_stderr, tcp_timestamps, stdio_timestamps, total_bytes, duration })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_management::SessionStatus;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_get_session() {
        let dir = TempDir::new().unwrap();
        let storage = FileStorage::new(dir.path()).unwrap();
        let session = Session {
            id: Uuid::new_v4(),
            service_name: "ssh".into(),
            client_addr: "127.0.0.1:2222".parse().unwrap(),
            start_time: Utc::now(),
            end_time: Some(Utc::now()),
            container_id: Some("cont-1".into()),
            bytes_transferred: 42,
            status: SessionStatus::Completed,
        };
        storage.save_session(&session).unwrap();
        let all = storage.get_sessions(None).unwrap();
        assert!(all.iter().any(|s| s.id == session.id));

        let filtered = storage.get_sessions(Some(SessionFilter { service_name: Some("ssh".into()), ..Default::default() })).unwrap();
        assert!(filtered.iter().any(|s| s.id == session.id));

        let none = storage.get_sessions(Some(SessionFilter { service_name: Some("http".into()), ..Default::default() })).unwrap();
        assert!(!none.iter().any(|s| s.id == session.id));
    }

    #[test]
    fn test_interaction_data_roundtrip() {
        let dir = TempDir::new().unwrap();
        let storage = FileStorage::new(dir.path()).unwrap();
        let id = Uuid::new_v4();
        storage.save_interaction(id, b"hello ").unwrap();
        storage.save_interaction(id, b"world").unwrap();
        let data = storage.get_session_data(id).unwrap();
        assert_eq!(data, b"hello world");
    }

    #[test]
    fn test_capture_artifacts_roundtrip() {
        let dir = TempDir::new().unwrap();
        let storage = FileStorage::new(dir.path()).unwrap();
        let id = Uuid::new_v4();
        let now = Utc::now();
        let artifacts = CaptureArtifacts {
            session_id: id,
            tcp_client_to_container: b"c2s".to_vec(),
            tcp_container_to_client: b"s2c".to_vec(),
            stdio_stdin: b"in".to_vec(),
            stdio_stdout: b"out".to_vec(),
            stdio_stderr: b"err".to_vec(),
            tcp_timestamps: vec![(now, Direction::ClientToContainer, 3), (now, Direction::ContainerToClient, 3)],
            stdio_timestamps: vec![(now, StdioStream::Stdout, 3)],
            total_bytes: 9,
            duration: chrono::Duration::seconds(5),
        };
        storage.save_capture_artifacts(&artifacts).unwrap();
        let got = storage.get_capture_artifacts(id).unwrap();
        assert_eq!(got.session_id, artifacts.session_id);
        assert_eq!(got.tcp_client_to_container, artifacts.tcp_client_to_container);
        assert_eq!(got.tcp_container_to_client, artifacts.tcp_container_to_client);
        assert_eq!(got.stdio_stdout, artifacts.stdio_stdout);
        assert_eq!(got.total_bytes, artifacts.total_bytes);
        assert_eq!(got.duration, artifacts.duration);
    }
}

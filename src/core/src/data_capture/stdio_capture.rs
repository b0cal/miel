use std::io::BufRead;
use std::io::{self, Read};
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use log::{debug, trace, warn};
use uuid::Uuid;

use super::types::StdioStream;
use crate::error_handling::types::CaptureError;

type StdioTimestamps = Vec<(DateTime<Utc>, StdioStream, usize)>;
type StdioArtifacts = (Vec<u8>, Vec<u8>, Vec<u8>, StdioTimestamps);

#[derive(Debug)]
pub struct StdioCapture {
    pub(crate) session_id: Uuid,
    pub(crate) stdin_data: Mutex<Vec<u8>>,
    pub(crate) stdout_data: Mutex<Vec<u8>>,
    pub(crate) stderr_data: Mutex<Vec<u8>>,
    pub(crate) timestamps: Mutex<StdioTimestamps>,
}

impl StdioCapture {
    pub fn new(session_id: Uuid) -> Self {
        debug!("[{}] StdioCapture created", session_id);
        Self {
            session_id,
            stdin_data: Mutex::new(Vec::new()),
            stdout_data: Mutex::new(Vec::new()),
            stderr_data: Mutex::new(Vec::new()),
            timestamps: Mutex::new(Vec::new()),
        }
    }

    pub fn capture_pty(&self, mut pty_master: std::fs::File) -> Result<(), CaptureError> {
        debug!("[{}] StdioCapture snapshot start", self.session_id);
        let mut buf = [0u8; 4096];
        match pty_master.read(&mut buf) {
            Ok(0) => {
                trace!("[{}] PTY read returned EOF", self.session_id);
            }
            Ok(n) => {
                self.stdout_data
                    .lock()
                    .unwrap()
                    .extend_from_slice(&buf[..n]);
                self.timestamps
                    .lock()
                    .unwrap()
                    .push((Utc::now(), StdioStream::Stdout, n));
                let preview = &buf[..std::cmp::min(n, 64)];
                trace!(
                    "[{}] captured STDOUT {} bytes: {}{}",
                    self.session_id,
                    n,
                    String::from_utf8_lossy(preview),
                    if n > 64 { " ..." } else { "" }
                );
            }
            Err(e) => {
                if e.kind() != io::ErrorKind::WouldBlock {
                    return Err(CaptureError::StdioError(e));
                } else {
                    trace!("[{}] PTY WouldBlock on snapshot", self.session_id);
                }
            }
        }
        Ok(())
    }

    /// Parse a unified container activity log file and split it into STDIN/STDOUT/STDERR streams.
    ///
    /// Supported line formats (examples):
    /// ```txt
    /// - "[YYYY-mm-dd HH:MM:SS UTC] [STDIN] <text>"
    /// - "[YYYY-mm-dd HH:MM:SS UTC] [STDOUT] <text>"
    /// - "[YYYY-mm-dd HH:MM:SS UTC] [STDERR] <text>"
    /// - "[YYYY-mm-dd HH:MM:SS UTC] [SSH-CMD] <text>" => mapped to STDIN
    /// - "[YYYY-mm-dd HH:MM:SS UTC] [SSH-OUTPUT] <text>" => mapped to STDOUT
    /// - "[YYYY-mm-dd HH:MM:SS UTC] [SSH-ERROR] <text>" => mapped to STDERR
    /// ```
    /// Other tags (e.g., SSHD, SSH-SESSION, SSH-EXIT, HTTP-INFO, HTTP-ERROR, HTTP-SERVER
    /// are ignored for byte streams.
    pub fn capture_activity_log_from_path<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> Result<(), CaptureError> {
        let path_ref = path.as_ref();
        debug!(
            "[{}] Parsing activity log from file: {}",
            self.session_id,
            path_ref.display()
        );
        let file = std::fs::File::open(path_ref).map_err(CaptureError::StdioError)?;
        let reader = std::io::BufReader::new(file);
        let mut total_lines = 0usize;
        for line_res in reader.lines() {
            match line_res {
                Ok(line) => {
                    total_lines += 1;
                    self.parse_activity_log_line(&line);
                }
                Err(e) => return Err(CaptureError::StdioError(e)),
            }
        }
        debug!(
            "[{}] Parsed {} activity log lines",
            self.session_id, total_lines
        );
        Ok(())
    }

    fn parse_activity_log_line(&self, line: &str) {
        // Skip header lines like === Container ...
        if line.starts_with("=== ") {
            trace!("[{}] skipping header line", self.session_id);
            return;
        }
        // Expect format: [timestamp] [SERVICE] [STREAM] content
        let mut rest = line;
        // Strip leading [timestamp]
        if let Some(close) = rest.find(']') {
            rest = rest[close + 1..].trim_start();
        } else {
            trace!(
                "[{}] unrecognized line (no timestamp): {}",
                self.session_id,
                line
            );
            return;
        }
        // First tag: [SERVICE]
        if !rest.starts_with('[') {
            trace!(
                "[{}] unrecognized line (no service tag): {}",
                self.session_id,
                line
            );
            return;
        }
        let service_end = match rest.find(']') {
            Some(i) => i,
            None => {
                trace!("[{}] malformed service tag: {}", self.session_id, line);
                return;
            }
        };
        let service = &rest[1..service_end];
        rest = rest[service_end + 1..].trim_start();

        // Second tag: [STREAM]
        if !rest.starts_with('[') {
            trace!(
                "[{}] missing stream tag after service [{}]",
                self.session_id,
                service
            );
            return;
        }
        let stream_end = match rest.find(']') {
            Some(i) => i,
            None => {
                trace!("[{}] malformed stream tag: {}", self.session_id, line);
                return;
            }
        };
        let stream_tag = &rest[1..stream_end];
        let content = rest[stream_end + 1..].trim_start();

        let stream = match stream_tag {
            "STDIN" => Some(StdioStream::Stdin),
            "STDOUT" => Some(StdioStream::Stdout),
            "STDERR" => Some(StdioStream::Stderr),
            _ => None,
        };

        if let Some(s) = stream {
            let mut bytes = content.as_bytes().to_vec();
            bytes.push(b'\n');
            let n = bytes.len();
            match s {
                StdioStream::Stdin => self.stdin_data.lock().unwrap().extend_from_slice(&bytes),
                StdioStream::Stdout => self.stdout_data.lock().unwrap().extend_from_slice(&bytes),
                StdioStream::Stderr => self.stderr_data.lock().unwrap().extend_from_slice(&bytes),
            }
            self.timestamps.lock().unwrap().push((Utc::now(), s, n));
            trace!(
                "[{}] parsed [{}] {} {} bytes: {}{}",
                self.session_id,
                service,
                match s {
                    StdioStream::Stdin => "STDIN",
                    StdioStream::Stdout => "STDOUT",
                    StdioStream::Stderr => "STDERR",
                },
                n,
                String::from_utf8_lossy(&bytes[..std::cmp::min(n, 64)]),
                if n > 64 { " ..." } else { "" }
            );
        } else {
            // Not a stdio data line; ignore but occasionally warn for unknown stream tags
            if service == "SSHD"
                || service == "CONTAINER"
                || service == "HTTP-SERVER"
                || service.ends_with("-INFO")
                || service.ends_with("-ERROR")
            {
                trace!(
                    "[{}] non-stdio service [{}] ignored",
                    self.session_id,
                    service
                );
            } else {
                warn!(
                    "[{}] unknown stream tag [{}] for service [{}]",
                    self.session_id, stream_tag, service
                );
            }
        }
    }

    pub fn get_artifacts(&self) -> StdioArtifacts {
        let i = self.stdin_data.lock().unwrap().clone();
        let o = self.stdout_data.lock().unwrap().clone();
        let e = self.stderr_data.lock().unwrap().clone();
        let t = self.timestamps.lock().unwrap().clone();
        (i, o, e, t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_demo_log_to_streams() {
        let _ = env_logger::builder().is_test(true).try_init();
        let demo = r#"=== Container miel-ssh-699296c7-aee4-4751-93d8-97ce46b79bcd Activity Log Started at 2025-09-03 18:17:07 UTC ===
[2025-09-03 20:17:07 UTC] [SSHD] /etc/ssh/sshd_config line 7: Deprecated option UsePrivilegeSeparation
[2025-09-03 20:17:17 UTC] [SSH] [STDIN] export PS1=\"miel@honeypot:\\w$ \"
[2025-09-03 20:17:18 UTC] [SSH] [STDIN] ls
[2025-09-03 20:17:18 UTC] [SSH] [STDIN] ls
[2025-09-03 20:17:18 UTC] [SSH] [STDOUT] total 0
[2025-09-03 20:17:18 UTC] [SSH] [STDERR] warning: something minor
[2025-09-03 20:17:19 UTC] [SSH] [STDIN] pwd
[2025-09-03 20:17:22 UTC] [SSH] [STDIN] exit
[2025-09-03 20:17:22 UTC] [SSHD] Received disconnect from 127.0.0.1 port 36414:11: disconnected by user
"#;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity.log");
        std::fs::write(&path, demo).unwrap();

        let cap = StdioCapture::new(Uuid::new_v4());
        cap.capture_activity_log_from_path(&path).unwrap();

        let (stdin_b, stdout_b, stderr_b, ts) = cap.get_artifacts();
        let stdin_s = String::from_utf8(stdin_b).unwrap();
        let stdout_s = String::from_utf8(stdout_b).unwrap();
        let stderr_s = String::from_utf8(stderr_b).unwrap();

        assert!(stdin_s.contains("ls\n"));
        assert!(stdin_s.contains("pwd\n"));
        assert!(stdin_s.contains("exit\n"));
        assert!(stdout_s.contains("total 0\n"));
        assert!(stderr_s.contains("warning: something minor\n"));
        assert!(!ts.is_empty());
    }

    #[test]
    fn parse_http_activity_log() {
        let _ = env_logger::builder().is_test(true).try_init();
        let http_log = r#"=== Container miel-http-96a584c9-a064-4b56-9e6a-e4fa07a3c598 Activity Log Started at 2025-09-03 20:32:43 UTC ===
[2025-09-03 20:32:43 UTC] [HTTP] [STDOUT] Server listening on 127.0.0.1:38959
[2025-09-03 20:34:05 UTC] [HTTP] [STDIN] GET / HTTP/1.1
"#;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("http_activity.log");
        std::fs::write(&path, http_log).unwrap();

        let cap = StdioCapture::new(Uuid::new_v4());
        cap.capture_activity_log_from_path(&path).unwrap();

        let (stdin_b, stdout_b, stderr_b, ts) = cap.get_artifacts();
        let stdin_s = String::from_utf8(stdin_b).unwrap();
        let stdout_s = String::from_utf8(stdout_b).unwrap();
        let stderr_s = String::from_utf8(stderr_b).unwrap();

        assert!(stdout_s.contains("Server listening on 127.0.0.1:38959\n"));
        assert!(stdin_s.contains("GET / HTTP/1.1\n"));
        assert!(stderr_s.is_empty());
        assert!(!ts.is_empty());
    }

    #[test]
    fn parse_ssh_activity_log_sample() {
        let _ = env_logger::builder().is_test(true).try_init();
        let ssh_log = r#"=== Container miel-ssh-1909fc67-86d2-473d-a0c0-7058c1bdbac5 Activity Log Started at 2025-09-03 20:32:42 UTC ===
[2025-09-03 22:32:43 UTC] [SSHD] /etc/ssh/sshd_config line 7: Deprecated option UsePrivilegeSeparation
[2025-09-03 22:32:43 UTC] [SSHD] Server listening on 127.0.0.1 port 45245.
[2025-09-03 22:32:43 UTC] [SSHD] rexec line 7: Deprecated option UsePrivilegeSeparation
[2025-09-03 22:32:43 UTC] [SSHD] WARNING: 'UsePAM no' is not supported in this build and may cause several problems.
[2025-09-03 22:32:43 UTC] [SSHD] Connection from 127.0.0.1 port 51952 on 127.0.0.1 port 45245 rdomain ""
[2025-09-03 22:32:45 UTC] [SSHD] rexec line 7: Deprecated option UsePrivilegeSeparation
[2025-09-03 22:32:45 UTC] [SSHD] WARNING: 'UsePAM no' is not supported in this build and may cause several problems.
[2025-09-03 22:32:45 UTC] [SSHD] Connection from 127.0.0.1 port 51968 on 127.0.0.1 port 45245 rdomain ""
[2025-09-03 22:32:48 UTC] [SSHD] Accepted password for miel from 127.0.0.1 port 51968 ssh2
[2025-09-03 22:32:48 UTC] [SSHD] User child is on pid 23
[2025-09-03 22:32:48 UTC] [SSHD] lastlog_openseek: Couldn't stat /var/log/lastlog: No such file or directory
[2025-09-03 22:32:48 UTC] [SSHD] lastlog_openseek: Couldn't stat /var/log/lastlog: No such file or directory
[2025-09-03 22:32:48 UTC] [SSH-SESSION] Interactive shell session started
[2025-09-03 22:32:48 UTC] [SSHD] Starting session: shell on pts/1 for miel from 127.0.0.1 port 51968 id 0
[2025-09-03 22:32:50 UTC] [SSH] [STDIN] ls
[2025-09-03 22:32:52 UTC] [SSH] [STDIN] cat
[2025-09-03 22:32:57 UTC] [SSH] [STDIN] w
[2025-09-03 22:32:57 UTC] [SSH] [STDOUT]  22:32:57 up  1:37,  0 user,  load average: 2.54, 4.02, 6.63
[2025-09-03 22:32:57 UTC] [SSH] [STDOUT] USER     TTY        LOGIN@   IDLE   JCPU   PCPU WHAT
[2025-09-03 22:33:12 UTC] [SSH] [STDIN] ls
[2025-09-03 22:33:19 UTC] [SSH] [STDIN] w
[2025-09-03 22:33:19 UTC] [SSH] [STDOUT]  22:33:19 up  1:38,  0 user,  load average: 1.96, 3.79, 6.50
[2025-09-03 22:33:19 UTC] [SSH] [STDOUT] USER     TTY        LOGIN@   IDLE   JCPU   PCPU WHAT
[2025-09-03 22:33:31 UTC] [SSHD] syslogin_perform_logout: logout() returned an error
[2025-09-03 22:33:31 UTC] [SSHD] Close session: user miel from 127.0.0.1 port 51968 id 0
[2025-09-03 22:33:31 UTC] [SSHD] Received disconnect from 127.0.0.1 port 51968:11: disconnected by user
[2025-09-03 22:33:31 UTC] [SSHD] Disconnected from user miel 127.0.0.1 port 51968
[2025-09-03 22:34:44 UTC] [SSHD] Timeout before authentication for connection from 127.0.0.1 to 127.0.0.1, pid = 13
[2025-09-03 22:34:44 UTC] [SSHD] srclimit_penalise: ipv4: new 127.0.0.1/32 active penalty of 90 seconds for penalty: exceeded LoginGraceTime
"#;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ssh_activity.log");
        std::fs::write(&path, ssh_log).unwrap();

        let cap = StdioCapture::new(Uuid::new_v4());
        cap.capture_activity_log_from_path(&path).unwrap();

        let (stdin_b, stdout_b, stderr_b, ts) = cap.get_artifacts();
        let stdin_s = String::from_utf8(stdin_b).unwrap();
        let stdout_s = String::from_utf8(stdout_b).unwrap();
        let stderr_s = String::from_utf8(stderr_b).unwrap();

        // Check that several commands were captured
        assert!(stdin_s.contains("ls \n") || stdin_s.contains("ls\n"));
        assert!(stdin_s.contains("w \n") || stdin_s.contains("w\n"));
        assert!(stdin_s.contains("cat \n") || stdin_s.contains("cat\n"));
        // Check that expected stdout snippets were captured
        assert!(stdout_s.contains("USER     TTY"));
        assert!(stdout_s.contains("up  1:"));
        // This sample contains no explicit STDERR lines
        assert!(stderr_s.is_empty());
        assert!(!ts.is_empty());
    }

    #[test]
    fn parse_noise_and_unknown_tags_ignored() {
        let _ = env_logger::builder().is_test(true).try_init();
        let log = r#"=== Container miel-mixed Activity Log Started at 2025-09-03 20:32:43 UTC ===
[2025-09-03 20:32:43 UTC] [SSHD] Server listening on 127.0.0.1 port 45245.
[2025-09-03 20:32:44 UTC] [HTTP-SERVER] [INFO] Started
[2025-09-03 20:32:45 UTC] [SSH-SESSION] Interactive shell session started
[2025-09-03 20:32:46 UTC] [SSH] [STDIN] echo hello
[2025-09-03 20:32:47 UTC] [FOO] [BAR] should be ignored
[2025-09-03 20:32:48 UTC] [SSH] [STDOUT] world
"#;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mixed_activity.log");
        std::fs::write(&path, log).unwrap();

        let cap = StdioCapture::new(Uuid::new_v4());
        cap.capture_activity_log_from_path(&path).unwrap();

        let (stdin_b, stdout_b, stderr_b, ts) = cap.get_artifacts();
        let stdin_s = String::from_utf8(stdin_b).unwrap();
        let stdout_s = String::from_utf8(stdout_b).unwrap();
        let stderr_s = String::from_utf8(stderr_b).unwrap();

        assert!(stdin_s.contains("echo hello\n"));
        assert!(stdout_s.contains("world\n"));
        assert!(stderr_s.is_empty());
        assert!(!ts.is_empty());
    }
}

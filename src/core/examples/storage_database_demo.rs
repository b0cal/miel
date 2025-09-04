use chrono::{Duration, Utc};
use env_logger::Env;
use log::{info, warn};
use miel::session::Session;
use miel::session_management::SessionStatus;
use miel::storage::database_storage::DatabaseStorage;
use miel::storage::file_storage::FileStorage;
use miel::storage::storage_trait::Storage;
use miel::data_capture::CaptureArtifacts;
use std::env;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

fn main() {
    // Initialize logger (RUST_LOG can override; default to info)
    let _ = env_logger::Builder::from_env(Env::default().default_filter_or("info")).try_init();

    // Choose an output directory for exports (does not affect backend env defaults)
    let out_dir: PathBuf = env::var("STORAGE_DEMO_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            env::current_dir()
                .expect("cwd")
                .join("target")
                .join("storage_demo")
        });
    fs::create_dir_all(&out_dir).expect("create output dir");

    // Backends: prefer environment variables if present
    let storage_db = if env::var("MIEL_DB_PATH").is_ok() {
        info!("Using DatabaseStorage::new() with MIEL_DB_PATH");
        DatabaseStorage::new().expect("create db (env)")
    } else {
        let db_path = out_dir.join("storage_demo.sqlite3");
        info!(
            "Using DatabaseStorage at {} (no MIEL_DB_PATH)",
            db_path.display()
        );
        DatabaseStorage::new_file(&db_path).expect("create db (file)")
    };

    let storage_fs = if env::var("MIEL_FILE_STORAGE_DIR").is_ok() {
        info!("Using FileStorage::new_default() with MIEL_FILE_STORAGE_DIR");
        FileStorage::new_default().expect("create file storage (env)")
    } else {
        info!(
            "Using FileStorage rooted at {} (no MIEL_FILE_STORAGE_DIR)",
            out_dir.display()
        );
        FileStorage::new(&out_dir).expect("create file storage (dir)")
    };

    // Create and save a session
    let session_id = Uuid::new_v4();
    let now = Utc::now();
    let sess = Session {
        id: session_id,
        service_name: "demo_service".to_string(),
        client_addr: "127.0.0.1:4444".parse().unwrap(),
        start_time: now,
        end_time: Some(now + Duration::seconds(1)),
        container_id: Some("container-123".to_string()),
        bytes_transferred: 42,
        status: SessionStatus::Completed,
    };
    storage_db.save_session(&sess).expect("save session db");
    storage_fs.save_session(&sess).expect("save session fs");
    info!("Saved session {} to DB and FS", sess.id);

    // Append some interaction data to both backends
    storage_db
        .save_interaction(session_id, b"Hello, ")
        .expect("save interaction db 1");
    storage_db
        .save_interaction(session_id, b"world!\n")
        .expect("save interaction db 2");

    storage_fs
        .save_interaction(session_id, b"Hello, ")
        .expect("save interaction fs 1");
    storage_fs
        .save_interaction(session_id, b"world!\n")
        .expect("save interaction fs 2");

    // Load and display interaction data from both backends
    let data_db = storage_db
        .get_session_data(session_id)
        .expect("get data db");
    let data_fs = storage_fs
        .get_session_data(session_id)
        .expect("get data fs");
    info!(
        "DB interaction data ({} bytes): {}",
        data_db.len(),
        String::from_utf8_lossy(&data_db)
    );
    info!(
        "FS interaction data ({} bytes): {}",
        data_fs.len(),
        String::from_utf8_lossy(&data_fs)
    );

    // Also export DB data to files for inspection (.bin + .txt)
    let data_path = out_dir.join(format!("session_{}_data.bin", session_id));
    fs::write(&data_path, &data_db).expect("write data file");
    let text_path = out_dir.join(format!("session_{}_data.txt", session_id));
    fs::write(&text_path, String::from_utf8_lossy(&data_db).as_bytes()).expect("write text file");
    info!(
        "Exported DB data to {} and {}",
        data_path.display(),
        text_path.display()
    );

    // Save some capture artifacts to both backends
    let arts = CaptureArtifacts {
        session_id,
        tcp_client_to_container: vec![1, 2, 3],
        tcp_container_to_client: vec![4, 5, 6],
        stdio_stdin: b"input".to_vec(),
        stdio_stdout: b"output".to_vec(),
        stdio_stderr: b"error".to_vec(),
        tcp_timestamps: vec![],
        stdio_timestamps: vec![],
        total_bytes: 6,
        duration: Duration::seconds(1),
    };
    storage_db
        .save_capture_artifacts(&arts)
        .expect("save artifacts db");
    storage_fs
        .save_capture_artifacts(&arts)
        .expect("save artifacts fs");

    // Load artifacts from both backends and display quick summary
    let fetched_db = storage_db
        .get_capture_artifacts(session_id)
        .expect("fetch artifacts db");
    let fetched_fs = storage_fs
        .get_capture_artifacts(session_id)
        .expect("fetch artifacts fs");
    info!(
        "Artifacts -> DB total_bytes={}, FS total_bytes={}",
        fetched_db.total_bytes, fetched_fs.total_bytes
    );
    if fetched_db.stdio_stdout != fetched_fs.stdio_stdout {
        warn!(
            "DB and FS stdout differ (sizes: DB={}, FS={})",
            fetched_db.stdio_stdout.len(),
            fetched_fs.stdio_stdout.len()
        );
    }

    // Export DB artifacts to a JSON file for easy inspection
    let artifacts_path = out_dir.join(format!("artifacts_{}.json", session_id));
    let json = serde_json::to_string_pretty(&fetched_db).expect("serialize artifacts");
    fs::write(&artifacts_path, json).expect("write artifacts json");
    info!(
        "Artifacts JSON written to {} (total_bytes={})",
        artifacts_path.display(),
        fetched_db.total_bytes
    );

    // Query sessions from both backends and display counts
    let all_db = storage_db.get_sessions(None).expect("list sessions db");
    let all_fs = storage_fs.get_sessions(None).expect("list sessions fs");
    info!(
        "Total sessions -> DB: {}, FS: {}",
        all_db.len(),
        all_fs.len()
    );

    // Print detailed session info from DB
    for s in &all_db {
        info!(
            "DB session: id={} service={} client={} start={} end={:?} status={:?} bytes={}",
            s.id,
            s.service_name,
            s.client_addr,
            s.start_time,
            s.end_time,
            s.status,
            s.bytes_transferred
        );
    }
    // Print detailed session info from FS
    for s in &all_fs {
        info!(
            "FS session: id={} service={} client={} start={} end={:?} status={:?} bytes={}",
            s.id,
            s.service_name,
            s.client_addr,
            s.start_time,
            s.end_time,
            s.status,
            s.bytes_transferred
        );
    }

    // Artifacts content preview
    let stdout_db = String::from_utf8_lossy(&fetched_db.stdio_stdout);
    let preview = stdout_db.chars().take(64).collect::<String>();
    info!(
        "Artifacts preview (DB stdout, first <=64 chars): {}{}",
        preview,
        if fetched_db.stdio_stdout.len() > 64 {
            "…"
        } else {
            ""
        }
    );

    info!("Demo complete. Inspect files under: {}", out_dir.display());
}

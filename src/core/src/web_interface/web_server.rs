use std::net::SocketAddr;
use std::sync::Arc;

use super::routes::*;
use crate::error_handling::types::WebError;
use crate::session_management::session_manager::SessionManager;
use crate::storage::storage_trait::Storage;
use crate::storage::types::{CaptureArtifacts, Session};
use uuid::Uuid;

use warp::{http::StatusCode, reply, Filter, Rejection, Reply};

/// API error payload
#[derive(serde::Serialize)]
pub struct ApiError {
    pub message: String,
}

/// Web server for HTTP API and dashboard
pub struct WebServer {
    storage: Arc<dyn Storage + Send + Sync>,
    session_manager: Arc<SessionManager>,
}

impl WebServer {
    /// Create a new WebServer instance
    pub fn new(storage: Arc<dyn Storage>, session_manager: Arc<SessionManager>) -> Self {
        Self {
            storage,
            session_manager,
        }
    }

    /// Start the web server on the given port
    pub async fn start(&self, port: u16) -> Result<(), WebError> {
        let dashboard = dashboard_route();
        let list_sessions = list_sessions_route(self.storage.clone());
        let get_session_data = get_session_data_route(self.storage.clone());
        let download_artifacts = download_artifacts_route(self.storage.clone());

        // Compose routes
        let routes = dashboard
            .or(list_sessions)
            .or(get_session_data)
            .or(download_artifacts);

        // Start server (warp 0.4)
        let addr: SocketAddr = ([0, 0, 0, 0], port).into();
        warp::serve(routes).run(addr).await;

        Ok(())
    }

    // The following helpers match the UML names and can be expanded later if needed
    #[allow(dead_code)]
    async fn get_dashboard() -> impl Reply {
        reply::html("<h1>Miel</h1>")
    }

    #[allow(dead_code)]
    async fn get_sessions(&self) -> impl Reply {
        let list: Vec<Session> = self.storage.get_sessions(None).unwrap_or_default();
        reply::json(&list)
    }

    #[allow(dead_code)]
    async fn get_session_data(&self, id: Uuid) -> impl Reply {
        match self.storage.get_session_data(id) {
            Ok(bytes) => reply::with_header(bytes, "Content-Type", "application/octet-stream")
                .into_response(),
            Err(_) => reply::with_status(
                reply::json(&ApiError {
                    message: "Not found".into(),
                }),
                StatusCode::NOT_FOUND,
            )
            .into_response(),
        }
    }

    #[allow(dead_code)]
    async fn download_capture_artifacts(&self, id: Uuid) -> impl Reply {
        match self.storage.get_capture_artifacts(id) {
            Ok(art) => reply::json(&art).into_response(),
            Err(_) => reply::with_status(
                reply::json(&ApiError {
                    message: "Not found".into(),
                }),
                StatusCode::NOT_FOUND,
            )
            .into_response(),
        }
    }
}

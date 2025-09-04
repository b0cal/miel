use std::net::SocketAddr;
use std::sync::Arc;

use log::info;

use super::routes::*;
use crate::error_handling::types::WebError;
use crate::storage::storage_trait::Storage;

use warp::Filter;

/// API error payload
#[derive(serde::Serialize)]
pub struct ApiError {
    pub message: String,
}

/// Web server for HTTP API and dashboard
pub struct WebServer {
    storage: Arc<dyn Storage + Send + Sync>,
}

impl WebServer {
    /// Create a new WebServer instance
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
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

        let addr: SocketAddr = ([0, 0, 0, 0], port).into();

        info!("WebUI starting on port {}", port);

        //WARN: will crash the whole program if the web server cannot run
        warp::serve(routes).run(addr).await;

        Ok(())
    }
}

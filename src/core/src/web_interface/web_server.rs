use std::net::SocketAddr;
use std::sync::Arc;

use crate::error_handling::types::WebError;
use crate::session_management::session_manager::SessionManager;
use crate::storage::storage_trait::Storage;
use crate::storage::types::{CaptureArtifacts, Session};
use uuid::Uuid;

use warp::{http::StatusCode, reply, Filter, Rejection, Reply};

/// API error payload
#[derive(serde::Serialize)]
struct ApiError {
    message: String,
}

/// Web server for HTTP API and dashboard
pub struct WebServer {
    storage: Arc<dyn Storage>,
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
        // Clone shared deps into filters
        let storage = self.storage.clone();
        let storage_for_artifacts = self.storage.clone();
        let storage_for_data = self.storage.clone();

        // GET / -> dashboard
        let dashboard = warp::path::end().and(warp::get()).and_then(|| async move {
            let html = r#"<html><head><title>Miel Dashboard</title></head>
                <body><h1>Miel is running</h1><p>See /sessions for JSON.</p></body></html>"#;
            Ok::<_, Rejection>(reply::html(html))
        });

        // GET /sessions -> list sessions
        let list_sessions = warp::path("sessions")
            .and(warp::path::end())
            .and(warp::get())
            .and_then(move || {
                let storage = storage.clone();
                async move {
                    match storage.get_sessions(None) {
                        Ok(list) => Ok::<_, Rejection>(reply::with_status(
                            reply::json(&list),
                            StatusCode::OK,
                        )),
                        Err(_) => Ok::<_, Rejection>(reply::with_status(
                            reply::json(&ApiError {
                                message: "Failed to load sessions".to_string(),
                            }),
                            StatusCode::INTERNAL_SERVER_ERROR,
                        )),
                    }
                }
            });

        // GET /sessions/:id/data -> raw bytes of interaction data
        let get_session_data = warp::path!("sessions" / String / "data")
            .and(warp::get())
            .and_then(move |id_str: String| {
                let storage = storage_for_data.clone();
                async move {
                    let id = match Uuid::parse_str(&id_str) {
                        Ok(u) => u,
                        Err(_) => {
                            let res = reply::with_status(
                                reply::json(&ApiError {
                                    message: "Invalid session id".to_string(),
                                }),
                                StatusCode::BAD_REQUEST,
                            )
                            .into_response();
                            return Ok::<_, Rejection>(res);
                        }
                    };

                    match storage.get_session_data(id) {
                        Ok(bytes) => {
                            let res = reply::with_status(
                                reply::with_header(
                                    bytes,
                                    "Content-Type",
                                    "application/octet-stream",
                                ),
                                StatusCode::OK,
                            )
                            .into_response();
                            Ok::<_, Rejection>(res)
                        }
                        Err(_) => {
                            let res = reply::with_status(
                                reply::json(&ApiError {
                                    message: "Session data not found".to_string(),
                                }),
                                StatusCode::NOT_FOUND,
                            )
                            .into_response();
                            Ok::<_, Rejection>(res)
                        }
                    }
                }
            });

        // GET /sessions/:id/artifacts -> capture artifacts as JSON (downloadable)
        let download_artifacts = warp::path!("sessions" / String / "artifacts")
            .and(warp::get())
            .and_then(move |id_str: String| {
                let storage = storage_for_artifacts.clone();
                async move {
                    let id = match Uuid::parse_str(&id_str) {
                        Ok(u) => u,
                        Err(_) => {
                            return Ok::<_, Rejection>(reply::with_status(
                                reply::json(&ApiError {
                                    message: "Invalid session id".to_string(),
                                }),
                                StatusCode::BAD_REQUEST,
                            ))
                        }
                    };

                    match storage.get_capture_artifacts(id) {
                        Ok(artifacts) => Ok::<_, Rejection>(reply::with_status(
                            reply::json(&artifacts),
                            StatusCode::OK,
                        )),
                        Err(_) => Ok::<_, Rejection>(reply::with_status(
                            reply::json(&ApiError {
                                message: "Artifacts not found".to_string(),
                            }),
                            StatusCode::NOT_FOUND,
                        )),
                    }
                }
            });

        // Compose routes
        let routes = dashboard
            .or(list_sessions)
            .or(get_session_data)
            .or(download_artifacts);

        // Start server (warp 0.4)
        let addr: SocketAddr = ([0, 0, 0, 0], port).into();
        warp::Server::bind(&addr).run(routes).await;

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

use crate::storage::types::SessionFilter;
use std::sync::Arc;
use uuid::Uuid;
use warp::{http::StatusCode, reply, Filter, Rejection, Reply};

use super::ApiError;
use crate::storage::storage_trait::Storage;

/// GET /
pub fn dashboard_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let webui_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../webui/dist");

    let redirect_root = warp::path::end()
        .and(warp::get())
        .map(|| warp::redirect::temporary(warp::http::Uri::from_static("/index.html")));

    let static_files = warp::fs::dir(webui_path);

    redirect_root.or(static_files)
}

/// GET /sessions
pub fn list_sessions_route(
    storage: Arc<dyn Storage + Send + Sync>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("api" / "sessions")
        .and(warp::path::end())
        .and(warp::get())
        .and(warp::query::<SessionFilter>())
        .and_then(move |filter: SessionFilter| {
            let storage = storage.clone();
            async move {
                match storage.get_sessions(Some(filter)) {
                    Ok(list) => {
                        // Directly return storage sessions
                        Ok::<_, Rejection>(warp::reply::with_status(
                            warp::reply::json(&list),
                            StatusCode::OK,
                        ))
                    }
                    Err(_) => Ok::<_, Rejection>(warp::reply::with_status(
                        warp::reply::json(&ApiError {
                            message: "Failed to load sessions".to_string(),
                        }),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )),
                }
            }
        })
}

/// GET /sessions/:id/data
pub fn get_session_data_route(
    storage: Arc<dyn Storage + Send + Sync>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("api" / "sessions" / String / "data")
        .and(warp::get())
        .and_then(move |id_str: String| {
            let storage = storage.clone();
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
                            reply::with_header(bytes, "Content-Type", "application/octet-stream"),
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
        })
}

/// GET /sessions/:id/artifacts
pub fn download_artifacts_route(
    storage: Arc<dyn Storage + Send + Sync>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("api" / "sessions" / String / "artifacts")
        .and(warp::get())
        .and_then(move |id_str: String| {
            let storage = storage.clone();
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
        })
}

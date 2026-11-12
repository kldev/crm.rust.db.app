//! Health probes and the React app (built `frontend/dist`, served from CRM_STATIC_DIR).

use std::path::{Component, PathBuf};

use axum::{
    Json,
    body::Body,
    extract::State,
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::{AppState, db};

/// Database readiness through the same path as every other call: `app.health_ping()`.
async fn database_up(state: &AppState) -> bool {
    let Ok(client) = state.db.get().await else {
        return false;
    };
    db::query_one(&client, "SELECT ok FROM app.health_ping()", &[])
        .await
        .map(|r| r.get::<_, bool>("ok"))
        .unwrap_or(false)
}

fn status(up: bool, components: serde_json::Value) -> Response {
    let code = if up {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let body = json!({ "status": if up { "UP" } else { "DOWN" }, "components": components });
    (code, Json(body)).into_response()
}

/// Readiness: the database and the attachments bucket (without it a ticket page cannot show its files).
pub async fn health(State(state): State<AppState>) -> Response {
    let (db, storage) = tokio::join!(database_up(&state), state.storage.ping());
    let up = |ok: bool| if ok { "UP" } else { "DOWN" };
    status(
        db && storage,
        json!({ "database": { "status": up(db) }, "storage": { "status": up(storage) } }),
    )
}

pub async fn liveness() -> Response {
    status(true, json!({}))
}

/// The paths that belong to the server; an unknown one is a 404, never the web app.
fn is_server_path(path: &str) -> bool {
    ["/api", "/actuator", "/v3", "/swagger-ui"]
        .iter()
        .any(|p| path == *p || path.starts_with(&format!("{p}/")))
}

fn content_type(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
    {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// GET of anything else: a file of the web app (a path with a dot), or index.html for its client-side routes,
/// so a reload of /clients/42 works (SpaForwardController.kt).
pub async fn spa(State(state): State<AppState>, uri: Uri) -> Response {
    let path = uri.path();
    if is_server_path(path) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let root = PathBuf::from(&state.config.static_dir);
    let relative = path.trim_start_matches('/');
    let last = relative.rsplit('/').next().unwrap_or_default();
    let file = if last.contains('.') {
        let candidate = PathBuf::from(relative);
        if candidate
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        {
            return StatusCode::NOT_FOUND.into_response();
        }
        root.join(candidate)
    } else {
        root.join("index.html")
    };
    match tokio::fs::read(&file).await {
        Ok(bytes) => {
            // Hashed assets never change; index.html must be fetched again after a deploy.
            let cache = if relative.starts_with("assets/") {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            };
            (
                [
                    (header::CONTENT_TYPE, content_type(&file)),
                    (header::CACHE_CONTROL, cache),
                ],
                Body::from(bytes),
            )
                .into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn server_paths_are_not_the_web_app() {
        assert!(super::is_server_path("/api/nope"));
        assert!(super::is_server_path("/v3/api-docs"));
        assert!(!super::is_server_path("/apis"));
        assert!(!super::is_server_path("/clients/42"));
    }
}

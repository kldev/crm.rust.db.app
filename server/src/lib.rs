//! CRM API: hand-written endpoints over PostgreSQL procedures, functions and views (ADR-0012, ADR-0013).

pub mod auth;
pub mod charts;
pub mod clients;
pub mod config;
pub mod db;
pub mod desk;
pub mod error;
pub mod exports;
pub mod migrate;
pub mod projects;
pub mod reports;
pub mod seed;
pub mod shared;
pub mod storage;
pub mod teams;
pub mod users;
pub mod web;

use std::sync::Arc;

use axum::{Router, middleware, response::Redirect, routing::get};
use deadpool_postgres::Pool;
use tower_sessions::{Expiry, SessionManagerLayer, cookie::SameSite};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

use crate::{auth::session::MemorySessions, config::Config};

#[derive(Clone)]
pub struct AppState {
    pub db: Pool,
    pub config: Arc<Config>,
    pub public_reports: Arc<desk::PublicReportLimiter>,
    pub storage: storage::Storage,
}

impl AppState {
    pub fn new(db: Pool, config: Arc<Config>) -> Self {
        let public_reports = Arc::new(desk::PublicReportLimiter::new(
            config.desk_public_reports_per_minute,
        ));
        let storage = storage::Storage::new(&config.storage);
        AppState {
            db,
            config,
            public_reports,
            storage,
        }
    }
}

#[derive(OpenApi)]
#[openapi(info(
    title = "CRM API",
    version = "v1",
    description = "Hand-written endpoints; each calls one \
    procedure or reads one function or view of schema app."
))]
struct ApiDoc;

/// The whole HTTP application: API, OpenAPI, health and the web app.
pub fn router(state: AppState, sessions: MemorySessions) -> Router {
    let (api, openapi) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(auth::router())
        .merge(users::router())
        .merge(clients::router())
        .merge(teams::router())
        .merge(projects::router())
        .merge(desk::router())
        .merge(reports::router())
        .merge(charts::router())
        .split_for_parts();

    let mut app = api
        .route("/actuator/health", get(web::health))
        .route("/actuator/health/readiness", get(web::health))
        .route("/actuator/health/liveness", get(web::liveness));
    app = if state.config.swagger_ui {
        app.merge(SwaggerUi::new("/swagger-ui").url("/v3/api-docs", openapi))
            .route(
                "/swagger-ui.html",
                get(|| async { Redirect::permanent("/swagger-ui/") }),
            )
    } else {
        app.route(
            "/v3/api-docs",
            get(move || async move { axum::Json(openapi) }),
        )
    };

    let secs = state.config.auth.session_timeout.as_secs() as i64;
    let session_layer = SessionManagerLayer::new(sessions)
        .with_name(auth::session::SESSION_COOKIE)
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_secure(state.config.auth.cookie_secure)
        .with_path("/")
        .with_expiry(Expiry::OnInactivity(
            tower_sessions::cookie::time::Duration::seconds(secs),
        ));

    app.fallback(web::spa)
        .layer(middleware::from_fn(auth::session::csrf_guard))
        .layer(session_layer)
        .with_state(state)
}

/// CRM_MODE=app: the pool, the first administrator, then HTTP until SIGTERM / Ctrl+C.
pub async fn serve(config: Config) -> anyhow::Result<()> {
    let config = Arc::new(config);
    let state = AppState::new(db::app_pool(&config.db)?, config.clone());
    auth::ensure_admin(&state).await?;
    let sessions = MemorySessions::default();
    sessions.start_cleanup();
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!(port = config.port, "listening");
    axum::serve(listener, router(state, sessions))
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}

async fn shutdown() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = term => {} }
    tracing::info!("shutting down");
}

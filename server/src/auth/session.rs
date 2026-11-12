//! Session and CSRF (ADR-0014): the logged-in user lives in a server-side session (cookie `JSESSIONID`,
//! HttpOnly, SameSite=Lax); writes carry the `XSRF-TOKEN` cookie value back in the `X-XSRF-TOKEN` header.

use std::{collections::HashMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use axum::{
    extract::{FromRequestParts, Request},
    http::{HeaderMap, Method, StatusCode, header, request::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tower_sessions::{
    Session,
    cookie::time::OffsetDateTime,
    session::{Id, Record},
    session_store::{self, SessionStore},
};

use crate::error::ApiError;

pub const SESSION_COOKIE: &str = "JSESSIONID";
pub const CSRF_COOKIE: &str = "XSRF-TOKEN";
pub const CSRF_HEADER: &str = "X-XSRF-TOKEN";
const USER_KEY: &str = "user";

/// In-process sessions (one instance, like the Spring version), with expired ones removed every minute.
#[derive(Clone, Debug, Default)]
pub struct MemorySessions(Arc<Mutex<HashMap<Id, Record>>>);

impl MemorySessions {
    pub fn start_cleanup(&self) {
        let sessions = self.0.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(60));
            loop {
                tick.tick().await;
                let now = OffsetDateTime::now_utc();
                sessions.lock().await.retain(|_, r| r.expiry_date > now);
            }
        });
    }
}

#[async_trait]
impl SessionStore for MemorySessions {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        let mut map = self.0.lock().await;
        while map.contains_key(&record.id) {
            record.id = Id::default();
        }
        map.insert(record.id, record.clone());
        Ok(())
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        self.0.lock().await.insert(record.id, record.clone());
        Ok(())
    }

    async fn load(&self, id: &Id) -> session_store::Result<Option<Record>> {
        let now = OffsetDateTime::now_utc();
        Ok(self
            .0
            .lock()
            .await
            .get(id)
            .filter(|r| r.expiry_date > now)
            .cloned())
    }

    async fn delete(&self, id: &Id) -> session_store::Result<()> {
        self.0.lock().await.remove(id);
        Ok(())
    }
}

/// The logged-in user kept in the session. `id` is the actor passed to every procedure (`p_actor_id`);
/// the procedures re-check the current role and active flag on each call.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionUser {
    pub id: i32,
    pub login: String,
    pub display_name: String,
    pub role: String,
}

pub async fn start(session: &Session, user: SessionUser) -> Result<(), ApiError> {
    // Session fixation protection: the session id changes on login.
    session
        .cycle_id()
        .await
        .map_err(|e| anyhow::anyhow!("session: {e}"))?;
    session
        .insert(USER_KEY, user)
        .await
        .map_err(|e| anyhow::anyhow!("session: {e}"))?;
    Ok(())
}

/// The authenticated user; 401 (empty body, as Spring's entry point) without a session.
pub struct Actor(pub SessionUser);

impl<S: Send + Sync> FromRequestParts<S> for Actor {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::unauthorized())?;
        match session.get::<SessionUser>(USER_KEY).await {
            Ok(Some(user)) => Ok(Actor(user)),
            _ => Err(ApiError::unauthorized()),
        }
    }
}

/// Coarse role rules of the endpoints (SecurityConfig.kt); the real rules are in the procedures.
/// Checked before the body is read, so a wrong role is 403 even for an invalid body.
pub struct Admin(pub SessionUser);

pub struct AdminOrManager(pub SessionUser);

impl<S: Send + Sync> FromRequestParts<S> for Admin {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Actor(user) = Actor::from_request_parts(parts, state).await?;
        if user.role == "ADMIN" {
            Ok(Admin(user))
        } else {
            Err(ApiError::forbidden())
        }
    }
}

impl<S: Send + Sync> FromRequestParts<S> for AdminOrManager {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Actor(user) = Actor::from_request_parts(parts, state).await?;
        if user.role == "ADMIN" || user.role == "MANAGER" {
            Ok(AdminOrManager(user))
        } else {
            Err(ApiError::forbidden())
        }
    }
}

pub fn cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(k, _)| *k == name)
        .map(|(_, v)| v.to_owned())
        .filter(|v| !v.is_empty())
}

/// Every write (POST, PUT, PATCH, DELETE) must send the XSRF-TOKEN cookie value as X-XSRF-TOKEN; 403 otherwise.
pub async fn csrf_guard(request: Request, next: Next) -> Response {
    let safe = matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
    );
    if !safe {
        let expected = cookie(request.headers(), CSRF_COOKIE);
        let sent = request
            .headers()
            .get(CSRF_HEADER)
            .and_then(|v| v.to_str().ok());
        let ok = matches!((expected.as_deref(), sent), (Some(e), Some(s)) if constant_eq(e, s));
        if !ok {
            return StatusCode::FORBIDDEN.into_response();
        }
    }
    next.run(request).await
}

fn constant_eq(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

/// `Set-Cookie` for the CSRF token: readable by the web app (not HttpOnly), whole site.
pub fn csrf_cookie(value: &str, secure: bool) -> String {
    let secure = if secure { "; Secure" } else { "" };
    format!("{CSRF_COOKIE}={value}; Path=/{secure}")
}

pub fn csrf_cookie_removal(secure: bool) -> String {
    let secure = if secure { "; Secure" } else { "" };
    format!("{CSRF_COOKIE}=; Path=/; Max-Age=0{secure}")
}

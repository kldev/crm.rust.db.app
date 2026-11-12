//! Identity (ADR-0014): login with JSON, the user in a server-side session, CSRF cookie, lockout after failed
//! logins. Users and password hashes are read and written only through `app` functions and procedures; the
//! hash is checked here (bcrypt), never in SQL.

pub mod session;

use axum::{
    Json,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    AppState, db,
    error::{ApiError, ApiResult},
    shared::{Body, Checks, Validate},
};
use session::{Actor, SessionUser};

pub const TAG: &str = "Auth";

/// bcrypt cost of new hashes (Spring's default strength).
pub const BCRYPT_COST: u32 = 10;

/// A hash compared against when the login is unknown, so both answers take about the same time.
static DUMMY_HASH: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(|| bcrypt::hash("not-a-password", BCRYPT_COST).expect("bcrypt works"));

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    pub login: Option<String>,
    pub password: Option<String>,
}

impl Validate for LoginRequest {
    fn validate(&self, v: &mut Checks) {
        v.text("login", &self.login, 1, 64);
        v.text("password", &self.password, 1, 200);
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    pub current_password: Option<String>,
    pub new_password: Option<String>,
}

impl Validate for ChangePasswordRequest {
    fn validate(&self, v: &mut Checks) {
        v.text("currentPassword", &self.current_password, 1, usize::MAX);
        v.text("newPassword", &self.new_password, 8, 200);
    }
}

#[derive(Deserialize, ToSchema)]
pub struct DevLoginRequest {
    pub login: Option<String>,
}

impl Validate for DevLoginRequest {
    fn validate(&self, v: &mut Checks) {
        v.text("login", &self.login, 1, 64);
    }
}

#[derive(Serialize, ToSchema)]
pub struct DevLoginStatus {
    pub enabled: bool,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MeResponse {
    pub id: i32,
    pub login: String,
    pub display_name: String,
    pub role: String,
}

impl From<&SessionUser> for MeResponse {
    fn from(u: &SessionUser) -> Self {
        MeResponse {
            id: u.id,
            login: u.login.clone(),
            display_name: u.display_name.clone(),
            role: u.role.clone(),
        }
    }
}

/// 401 "Invalid login or password." - one answer for every login failure.
fn invalid_login() -> ApiError {
    ApiError::Problem {
        status: StatusCode::UNAUTHORIZED,
        title: "Unauthorized",
        detail: "Invalid login or password.".into(),
        error_number: None,
    }
}

/// A bcrypt hash with the API's cost, for callers already off the async threads (the seed).
pub fn hash_password_blocking(password: &str) -> anyhow::Result<String> {
    Ok(bcrypt::hash(password, BCRYPT_COST)?)
}

pub async fn hash_password(password: String) -> ApiResult<String> {
    tokio::task::spawn_blocking(move || bcrypt::hash(password, BCRYPT_COST))
        .await
        .map_err(|e| anyhow::anyhow!(e))?
        .map_err(|e| ApiError::Internal(e.into()))
}

async fn verify_password(password: String, hash: String) -> ApiResult<bool> {
    Ok(
        tokio::task::spawn_blocking(move || bcrypt::verify(password, &hash).unwrap_or(false))
            .await
            .map_err(|e| anyhow::anyhow!(e))?,
    )
}

// ---- repository: one statement per function -------------------------------------------------------------

struct LoginUser {
    user: SessionUser,
    password_hash: String,
    active: bool,
    locked: bool,
}

async fn find_for_login(state: &AppState, login: &str) -> ApiResult<Option<LoginUser>> {
    let client = state.db.get().await?;
    let row = db::query_opt(
        &client,
        "SELECT id, login, display_name, role, password_hash, is_active, locked_until > now() AS locked \
         FROM app.user_get_for_login($1)",
        &[&login],
    )
    .await?;
    Ok(row.map(|r| LoginUser {
        user: SessionUser {
            id: r.get("id"),
            login: r.get("login"),
            display_name: r.get("display_name"),
            role: r.get("role"),
        },
        password_hash: r.get("password_hash"),
        active: r.get("is_active"),
        locked: r.get::<_, Option<bool>>("locked").unwrap_or(false),
    }))
}

async fn login_succeeded(state: &AppState, user_id: i32) -> ApiResult<()> {
    let client = state.db.get().await?;
    db::execute(&client, "CALL app.user_login_succeeded($1)", &[&user_id]).await
}

async fn login_failed(state: &AppState, login: &str) -> ApiResult<()> {
    let client = state.db.get().await?;
    db::execute(
        &client,
        "CALL app.user_login_failed($1, $2, $3)",
        &[
            &login,
            &state.config.auth.max_failed_logins,
            &state.config.auth.lock_minutes,
        ],
    )
    .await
}

pub async fn set_password(
    state: &AppState,
    actor_id: i32,
    user_id: i32,
    hash: &str,
) -> ApiResult<()> {
    let client = state.db.get().await?;
    db::execute(
        &client,
        "CALL app.user_set_password($1, $2, $3)",
        &[&actor_id, &user_id, &hash],
    )
    .await
}

/// Creates the first ADMIN from CRM_ADMIN_LOGIN / CRM_ADMIN_PASSWORD when the database has no administrator.
pub async fn ensure_admin(state: &AppState) -> anyhow::Result<()> {
    let (Some(login), Some(password)) = (
        state.config.auth.admin_login.clone(),
        state.config.auth.admin_password.clone(),
    ) else {
        tracing::info!("CRM_ADMIN_LOGIN / CRM_ADMIN_PASSWORD not set - no administrator bootstrap");
        return Ok(());
    };
    let hash = hash_password(password)
        .await
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let client = state.db.get().await?;
    let row = client
        .query_one(
            "CALL app.user_ensure_admin($1, $2, $3, NULL)",
            &[&login, &"Administrator", &hash],
        )
        .await?;
    if row.get::<_, bool>("p_created") {
        tracing::info!(login = %login, "administrator created");
    }
    Ok(())
}

// ---- endpoints -------------------------------------------------------------------------------------------

/// Sets the XSRF-TOKEN cookie. Send its value as the X-XSRF-TOKEN header with every write request.
#[utoipa::path(get, path = "/api/auth/csrf", tag = TAG, operation_id = "getCsrfToken",
    responses((status = 204, description = "Cookie set")))]
async fn get_csrf_token(State(state): State<AppState>, headers: axum::http::HeaderMap) -> Response {
    if session::cookie(&headers, session::CSRF_COOKIE).is_some() {
        return StatusCode::NO_CONTENT.into_response();
    }
    let token = uuid::Uuid::new_v4().to_string();
    (
        StatusCode::NO_CONTENT,
        [(
            header::SET_COOKIE,
            session::csrf_cookie(&token, state.config.auth.cookie_secure),
        )],
    )
        .into_response()
}

/// Log in with login and password; starts a session (cookie). 401 for any failure.
#[utoipa::path(post, path = "/api/auth/login", tag = TAG, operation_id = "login",
    request_body = LoginRequest, responses((status = 200, body = MeResponse)))]
async fn login(
    State(state): State<AppState>,
    session: Session,
    Body(body): Body<LoginRequest>,
) -> ApiResult<Json<MeResponse>> {
    let login = body.login.unwrap_or_default().trim().to_lowercase();
    let password = body.password.unwrap_or_default();
    let Some(found) = find_for_login(&state, &login).await? else {
        verify_password(password, DUMMY_HASH.clone()).await?;
        // Unknown login: counted like a wrong password (nothing to count), the same answer.
        login_failed(&state, &login).await?;
        return Err(invalid_login());
    };
    // Locked or deactivated: refused before the password is checked, and not counted.
    if found.locked || !found.active {
        return Err(invalid_login());
    }
    if !verify_password(password, found.password_hash).await? {
        login_failed(&state, &login).await?;
        return Err(invalid_login());
    }
    login_succeeded(&state, found.user.id).await?;
    let me = MeResponse::from(&found.user);
    session::start(&session, found.user).await?;
    Ok(Json(me))
}

/// Whether one-click development login is on (CRM_DEV_LOGIN). The login page shows it only then.
#[utoipa::path(get, path = "/api/auth/dev-login", tag = TAG, operation_id = "getDevLoginStatus",
    responses((status = 200, body = DevLoginStatus)))]
async fn get_dev_login_status(State(state): State<AppState>) -> Json<DevLoginStatus> {
    Json(DevLoginStatus {
        enabled: state.config.auth.dev_login,
    })
}

/// Local development only (CRM_DEV_LOGIN=true, otherwise 404): log in as an active user without a password.
/// Skips the password and the lockout, nothing else; roles apply as usual.
#[utoipa::path(post, path = "/api/auth/dev-login", tag = TAG, operation_id = "devLogin",
    request_body = DevLoginRequest, responses((status = 200, body = MeResponse)))]
async fn dev_login(
    State(state): State<AppState>,
    session: Session,
    Body(body): Body<DevLoginRequest>,
) -> ApiResult<Json<MeResponse>> {
    if !state.config.auth.dev_login {
        return Err(ApiError::Status(StatusCode::NOT_FOUND));
    }
    let login = body.login.unwrap_or_default().trim().to_lowercase();
    let found = find_for_login(&state, &login)
        .await?
        .filter(|u| u.active)
        .ok_or_else(invalid_login)?;
    let me = MeResponse::from(&found.user);
    session::start(&session, found.user).await?;
    Ok(Json(me))
}

/// Ends the session and drops the CSRF token. 204 also without a session.
#[utoipa::path(post, path = "/api/auth/logout", tag = TAG, operation_id = "logout",
    responses((status = 204, description = "Logged out")))]
async fn logout(State(state): State<AppState>, session: Session) -> ApiResult<Response> {
    session
        .flush()
        .await
        .map_err(|e| anyhow::anyhow!("session: {e}"))?;
    Ok((
        StatusCode::NO_CONTENT,
        [(
            header::SET_COOKIE,
            session::csrf_cookie_removal(state.config.auth.cookie_secure),
        )],
    )
        .into_response())
}

/// The logged-in user.
#[utoipa::path(get, path = "/api/auth/me", tag = TAG, operation_id = "getMe",
    responses((status = 200, body = MeResponse)))]
async fn get_me(Actor(user): Actor) -> Json<MeResponse> {
    Json(MeResponse::from(&user))
}

/// Change own password (requires the current one).
#[utoipa::path(post, path = "/api/auth/password", tag = TAG, operation_id = "changeOwnPassword",
    request_body = ChangePasswordRequest, responses((status = 204, description = "Changed")))]
async fn change_own_password(
    State(state): State<AppState>,
    Actor(user): Actor,
    Body(body): Body<ChangePasswordRequest>,
) -> ApiResult<StatusCode> {
    let current = find_for_login(&state, &user.login).await?;
    let ok = match current {
        Some(c) => {
            verify_password(body.current_password.unwrap_or_default(), c.password_hash).await?
        }
        None => false,
    };
    if !ok {
        return Err(ApiError::bad_request("Current password is not correct."));
    }
    let hash = hash_password(body.new_password.unwrap_or_default()).await?;
    set_password(&state, user.id, user.id, &hash).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_csrf_token))
        .routes(routes!(login))
        .routes(routes!(get_dev_login_status, dev_login))
        .routes(routes!(logout))
        .routes(routes!(get_me))
        .routes(routes!(change_own_password))
}

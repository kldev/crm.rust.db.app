//! User administration: ADMIN writes, ADMIN and MANAGER read (also enforced in the procedures).
//! Every write is one `CALL app.user_*`; the changed user is read back with `app.user_get` in the same transaction.

use axum::{Json, extract::State, http::StatusCode};
use chrono::{DateTime, Utc};
use deadpool_postgres::GenericClient;
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    AppState,
    auth::{
        hash_password,
        session::{Admin, AdminOrManager},
        set_password,
    },
    db,
    error::ApiResult,
    page_response,
    shared::{Body, Checks, Params, PathParam, Validate, cursor, page_size, ts},
};

pub const TAG: &str = "Users";
const ROLES: &[&str] = &["ADMIN", "MANAGER", "SALES"];

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub id: i32,
    pub login: String,
    pub display_name: String,
    pub role: String,
    pub active: bool,
    #[serde(serialize_with = "ts::local_opt")]
    pub locked_until: Option<DateTime<Utc>>,
    #[serde(serialize_with = "ts::local_opt")]
    pub last_login_at: Option<DateTime<Utc>>,
    #[serde(serialize_with = "ts::local")]
    pub created_at: DateTime<Utc>,
    #[serde(serialize_with = "ts::local")]
    pub updated_at: DateTime<Utc>,
    pub row_version: String,
}

page_response!(PageResponseUserResponse, UserResponse);

fn user_row(r: &Row) -> UserResponse {
    UserResponse {
        id: r.get("id"),
        login: r.get("login"),
        display_name: r.get("display_name"),
        role: r.get("role"),
        active: r.get("is_active"),
        locked_until: r.get("locked_until"),
        last_login_at: r.get("last_login_at"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
        row_version: r.get("row_version"),
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    pub login: Option<String>,
    pub display_name: Option<String>,
    pub role: Option<String>,
    pub password: Option<String>,
}

impl Validate for CreateUserRequest {
    fn validate(&self, v: &mut Checks) {
        v.text("login", &self.login, 3, 64);
        v.text("displayName", &self.display_name, 1, 200);
        v.one_of("role", &self.role, ROLES);
        v.text("password", &self.password, 8, 200);
    }
}

#[derive(Deserialize, ToSchema)]
pub struct ChangeRoleRequest {
    pub role: Option<String>,
}

impl Validate for ChangeRoleRequest {
    fn validate(&self, v: &mut Checks) {
        v.one_of("role", &self.role, ROLES);
    }
}

#[derive(Deserialize, ToSchema)]
pub struct SetActiveRequest {
    pub active: Option<bool>,
}

impl Validate for SetActiveRequest {
    fn validate(&self, v: &mut Checks) {
        v.present("active", &self.active);
    }
}

#[derive(Deserialize, ToSchema)]
pub struct ResetPasswordRequest {
    pub password: Option<String>,
}

impl Validate for ResetPasswordRequest {
    fn validate(&self, v: &mut Checks) {
        v.text("password", &self.password, 8, 200);
    }
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListUsersParams {
    pub role: Option<String>,
    pub cursor: Option<String>,
    /// 1-200, default 50.
    pub size: Option<i32>,
}

// ---- repository ------------------------------------------------------------------------------------------

async fn get<C: GenericClient + Sync>(
    client: &C,
    actor_id: i32,
    id: i32,
) -> ApiResult<UserResponse> {
    let row = db::query_one(
        client,
        "SELECT * FROM app.user_get($1, $2)",
        &[&actor_id, &id],
    )
    .await?;
    Ok(user_row(&row))
}

// ---- endpoints -------------------------------------------------------------------------------------------

/// Users page, ordered by id.
#[utoipa::path(get, path = "/api/users", tag = TAG, operation_id = "listUsers", params(ListUsersParams),
    responses((status = 200, body = PageResponseUserResponse)))]
async fn list_users(
    State(state): State<AppState>,
    AdminOrManager(actor): AdminOrManager,
    Params(q): Params<ListUsersParams>,
) -> ApiResult<Json<PageResponseUserResponse>> {
    let size = page_size(q.size)?;
    let after_id: Option<i32> =
        cursor::decode(q.cursor.as_deref(), 1)?.and_then(|p| p[0].parse().ok());
    let client = state.db.get().await?;
    let rows = db::query(
        &client,
        "SELECT * FROM app.user_list($1, $2, $3, $4)",
        &[&actor.id, &q.role, &after_id, &size],
    )
    .await?;
    let users = rows.iter().map(user_row).collect();
    Ok(Json(PageResponseUserResponse::new(cursor::page(
        users,
        size,
        |u| cursor::encode(&[&u.id]),
    ))))
}

/// Create a user.
#[utoipa::path(post, path = "/api/users", tag = TAG, operation_id = "createUser",
    request_body = CreateUserRequest, responses((status = 201, body = UserResponse)))]
async fn create_user(
    State(state): State<AppState>,
    Admin(actor): Admin,
    Body(body): Body<CreateUserRequest>,
) -> ApiResult<(StatusCode, Json<UserResponse>)> {
    let hash = hash_password(body.password.unwrap_or_default()).await?;
    let mut client = state.db.get().await?;
    let tx = client.transaction().await?;
    let row = db::query_one(
        &tx,
        "CALL app.user_create($1, $2, $3, $4, $5, NULL)",
        &[
            &actor.id,
            &body.login,
            &body.display_name,
            &body.role,
            &hash,
        ],
    )
    .await?;
    let user = get(&tx, actor.id, row.get("p_id")).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(user)))
}

/// Change the role of a user (not your own).
#[utoipa::path(put, path = "/api/users/{id}/role", tag = TAG, operation_id = "changeUserRole",
    params(("id" = i32, Path)), request_body = ChangeRoleRequest, responses((status = 200, body = UserResponse)))]
async fn change_user_role(
    State(state): State<AppState>,
    Admin(actor): Admin,
    PathParam(id): PathParam<i32>,
    Body(body): Body<ChangeRoleRequest>,
) -> ApiResult<Json<UserResponse>> {
    let mut client = state.db.get().await?;
    let tx = client.transaction().await?;
    db::execute(
        &tx,
        "CALL app.user_change_role($1, $2, $3)",
        &[&actor.id, &id, &body.role],
    )
    .await?;
    let user = get(&tx, actor.id, id).await?;
    tx.commit().await?;
    Ok(Json(user))
}

/// Activate or deactivate a user (not yourself).
#[utoipa::path(put, path = "/api/users/{id}/active", tag = TAG, operation_id = "setUserActive",
    params(("id" = i32, Path)), request_body = SetActiveRequest, responses((status = 200, body = UserResponse)))]
async fn set_user_active(
    State(state): State<AppState>,
    Admin(actor): Admin,
    PathParam(id): PathParam<i32>,
    Body(body): Body<SetActiveRequest>,
) -> ApiResult<Json<UserResponse>> {
    let mut client = state.db.get().await?;
    let tx = client.transaction().await?;
    db::execute(
        &tx,
        "CALL app.user_set_active($1, $2, $3)",
        &[&actor.id, &id, &body.active],
    )
    .await?;
    let user = get(&tx, actor.id, id).await?;
    tx.commit().await?;
    Ok(Json(user))
}

/// Set a new password for a user; clears a lockout.
#[utoipa::path(put, path = "/api/users/{id}/password", tag = TAG, operation_id = "resetUserPassword",
    params(("id" = i32, Path)), request_body = ResetPasswordRequest, responses((status = 204, description = "Changed")))]
async fn reset_user_password(
    State(state): State<AppState>,
    Admin(actor): Admin,
    PathParam(id): PathParam<i32>,
    Body(body): Body<ResetPasswordRequest>,
) -> ApiResult<StatusCode> {
    let hash = hash_password(body.password.unwrap_or_default()).await?;
    set_password(&state, actor.id, id, &hash).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_users, create_user))
        .routes(routes!(change_user_role))
        .routes(routes!(set_user_active))
        .routes(routes!(reset_user_password))
}

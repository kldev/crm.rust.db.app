//! Clients. Every write is one `CALL app.client_*`; reads are `app.client_search`, `app.client_count` and
//! `app.client_get`. Rules and visibility (SALES works only on own clients) live in the procedures.

use axum::{Json, extract::State, http::StatusCode};
use chrono::{DateTime, Utc};
use deadpool_postgres::GenericClient;
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    AppState,
    auth::session::Actor,
    db,
    error::{ApiError, ApiResult},
    page_response,
    shared::{Body, Checks, Params, PathParam, TotalResponse, Validate, cursor, page_size, ts},
};

pub const TAG: &str = "Clients";

/// Row of the clients list (without the note).
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClientSummary {
    pub id: i64,
    pub name: String,
    pub nip: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub city: Option<String>,
    pub status: String,
    pub assigned_user_id: Option<i32>,
    pub assigned_user_login: Option<String>,
    #[serde(serialize_with = "ts::local")]
    pub created_at: DateTime<Utc>,
    #[serde(serialize_with = "ts::local")]
    pub updated_at: DateTime<Utc>,
    pub row_version: String,
}

page_response!(PageResponseClientSummary, ClientSummary);

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClientResponse {
    pub id: i64,
    pub name: String,
    pub nip: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub city: Option<String>,
    pub status: String,
    pub assigned_user_id: Option<i32>,
    pub assigned_user_login: Option<String>,
    pub note: Option<String>,
    #[serde(serialize_with = "ts::local")]
    pub created_at: DateTime<Utc>,
    pub created_by: Option<i32>,
    #[serde(serialize_with = "ts::local")]
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<i32>,
    pub row_version: String,
}

/// Create and full update share the fields; the update also carries `rowVersion` read from the server.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClientRequest {
    pub name: Option<String>,
    pub nip: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub city: Option<String>,
    pub status: Option<String>,
    pub assigned_user_id: Option<i32>,
    pub note: Option<String>,
    pub row_version: Option<String>,
}

impl Validate for ClientRequest {
    fn validate(&self, v: &mut Checks) {
        v.text("name", &self.name, 1, 200);
        v.max("nip", &self.nip, 20);
        v.max("email", &self.email, 254);
        if let Some(email) = self.email.as_deref().filter(|e| !e.is_empty())
            && !email
                .split_once('@')
                .is_some_and(|(local, domain)| !local.is_empty() && !domain.is_empty())
        {
            v.fail("email", "must be a well-formed email address");
        }
        v.max("phone", &self.phone, 30);
        v.max("city", &self.city, 100);
        v.one_of("status", &self.status, &["LEAD", "ACTIVE", "INACTIVE"]);
        v.max("note", &self.note, 2000);
        v.row_version(&self.row_version, false);
    }
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct SearchClientsParams {
    pub name_prefix: Option<String>,
    pub nip: Option<String>,
    pub email: Option<String>,
    pub city: Option<String>,
    pub status: Option<String>,
    pub cursor: Option<String>,
    /// 1-200, default 50.
    pub size: Option<i32>,
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct CountClientsParams {
    pub name_prefix: Option<String>,
    pub nip: Option<String>,
    pub email: Option<String>,
    pub city: Option<String>,
    pub status: Option<String>,
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct DeleteClientParams {
    pub row_version: String,
}

fn summary_row(r: &Row) -> ClientSummary {
    ClientSummary {
        id: r.get("id"),
        name: r.get("name"),
        nip: r.get("nip"),
        email: r.get("email"),
        phone: r.get("phone"),
        city: r.get("city"),
        status: r.get("status"),
        assigned_user_id: r.get("assigned_user_id"),
        assigned_user_login: r.get("assigned_user_login"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
        row_version: r.get("row_version"),
    }
}

fn client_row(r: &Row) -> ClientResponse {
    ClientResponse {
        id: r.get("id"),
        name: r.get("name"),
        nip: r.get("nip"),
        email: r.get("email"),
        phone: r.get("phone"),
        city: r.get("city"),
        status: r.get("status"),
        assigned_user_id: r.get("assigned_user_id"),
        assigned_user_login: r.get("assigned_user_login"),
        note: r.get("note"),
        created_at: r.get("created_at"),
        created_by: r.get("created_by"),
        updated_at: r.get("updated_at"),
        updated_by: r.get("updated_by"),
        row_version: r.get("row_version"),
    }
}

fn blank_to_none(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.trim().is_empty())
}

// ---- repository ------------------------------------------------------------------------------------------

async fn get<C: GenericClient + Sync>(
    client: &C,
    actor_id: i32,
    id: i64,
) -> ApiResult<ClientResponse> {
    let row = db::query_one(
        client,
        "SELECT * FROM app.client_get($1, $2)",
        &[&actor_id, &id],
    )
    .await?;
    Ok(client_row(&row))
}

// ---- endpoints -------------------------------------------------------------------------------------------

/// Clients page. With `namePrefix` ordered by name, otherwise newest first. SALES users see only clients
/// assigned to them.
#[utoipa::path(get, path = "/api/clients", tag = TAG, operation_id = "searchClients",
    params(SearchClientsParams), responses((status = 200, body = PageResponseClientSummary)))]
async fn search_clients(
    State(state): State<AppState>,
    Actor(actor): Actor,
    Params(q): Params<SearchClientsParams>,
) -> ApiResult<Json<PageResponseClientSummary>> {
    let size = page_size(q.size)?;
    let name_prefix = blank_to_none(q.name_prefix);
    // Cursor = (id) when ordered by id, (id, name) when ordered by name.
    let by_name = name_prefix.is_some();
    let parts = cursor::decode(q.cursor.as_deref(), if by_name { 2 } else { 1 })?;
    let after_id: Option<i64> = match &parts {
        Some(p) => Some(
            p[0].parse()
                .map_err(|_| ApiError::bad_request("Invalid cursor."))?,
        ),
        None => None,
    };
    let after_name: Option<String> = parts.filter(|_| by_name).map(|p| p[1].clone());

    let client = state.db.get().await?;
    let rows = db::query(
        &client,
        "SELECT * FROM app.client_search($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        &[
            &actor.id,
            &name_prefix,
            &q.nip,
            &q.email,
            &q.city,
            &q.status,
            &after_id,
            &after_name,
            &size,
        ],
    )
    .await?;
    let items = rows.iter().map(summary_row).collect();
    Ok(Json(PageResponseClientSummary::new(cursor::page(
        items,
        size,
        |c| {
            if by_name {
                cursor::encode(&[&c.id, &c.name])
            } else {
                cursor::encode(&[&c.id])
            }
        },
    ))))
}

/// Number of clients matching the same filters as the list. Reads every matching row, so call it only when the
/// user asks for the total.
#[utoipa::path(get, path = "/api/clients/count", tag = TAG, operation_id = "countClients",
    params(CountClientsParams), responses((status = 200, body = TotalResponse)))]
async fn count_clients(
    State(state): State<AppState>,
    Actor(actor): Actor,
    Params(q): Params<CountClientsParams>,
) -> ApiResult<Json<TotalResponse>> {
    let client = state.db.get().await?;
    let row = db::query_one(
        &client,
        "SELECT total FROM app.client_count($1, $2, $3, $4, $5, $6)",
        &[
            &actor.id,
            &blank_to_none(q.name_prefix),
            &q.nip,
            &q.email,
            &q.city,
            &q.status,
        ],
    )
    .await?;
    Ok(Json(TotalResponse {
        total: row.get("total"),
    }))
}

/// Client details.
#[utoipa::path(get, path = "/api/clients/{id}", tag = TAG, operation_id = "getClient",
    params(("id" = i64, Path)), responses((status = 200, body = ClientResponse)))]
async fn get_client(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
) -> ApiResult<Json<ClientResponse>> {
    let client = state.db.get().await?;
    Ok(Json(get(&client, actor.id, id).await?))
}

/// Create a client. A SALES user creates clients assigned to themselves.
#[utoipa::path(post, path = "/api/clients", tag = TAG, operation_id = "createClient",
    request_body = ClientRequest, responses((status = 201, body = ClientResponse)))]
async fn create_client(
    State(state): State<AppState>,
    Actor(actor): Actor,
    Body(b): Body<ClientRequest>,
) -> ApiResult<(StatusCode, Json<ClientResponse>)> {
    let mut client = state.db.get().await?;
    let tx = client.transaction().await?;
    let row = db::query_one(
        &tx,
        "CALL app.client_create($1, $2, $3, $4, $5, $6, $7, $8, $9, NULL)",
        &[
            &actor.id,
            &b.name,
            &b.nip,
            &b.email,
            &b.phone,
            &b.city,
            &b.status,
            &b.assigned_user_id,
            &b.note,
        ],
    )
    .await?;
    let created = get(&tx, actor.id, row.get("p_id")).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(created)))
}

/// Update a client. `rowVersion` from the last read is required (409 when it changed).
#[utoipa::path(put, path = "/api/clients/{id}", tag = TAG, operation_id = "updateClient",
    params(("id" = i64, Path)), request_body = ClientRequest, responses((status = 200, body = ClientResponse)))]
async fn update_client(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
    Body(b): Body<ClientRequest>,
) -> ApiResult<Json<ClientResponse>> {
    let Some(row_version) = b.row_version.as_deref() else {
        return Err(ApiError::bad_request("rowVersion is required."));
    };
    let mut client = state.db.get().await?;
    let tx = client.transaction().await?;
    db::execute(
        &tx,
        "CALL app.client_update($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        &[
            &actor.id,
            &id,
            &b.name,
            &b.nip,
            &b.email,
            &b.phone,
            &b.city,
            &b.status,
            &b.assigned_user_id,
            &b.note,
            &row_version,
        ],
    )
    .await?;
    let updated = get(&tx, actor.id, id).await?;
    tx.commit().await?;
    Ok(Json(updated))
}

/// Delete a client without projects (MANAGER, ADMIN).
#[utoipa::path(delete, path = "/api/clients/{id}", tag = TAG, operation_id = "deleteClient",
    params(("id" = i64, Path), DeleteClientParams), responses((status = 204, description = "Deleted")))]
async fn delete_client(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
    Params(q): Params<DeleteClientParams>,
) -> ApiResult<StatusCode> {
    let mut checks = Checks::default();
    checks.row_version(&Some(q.row_version.clone()), true);
    checks.into_result()?;
    let client = state.db.get().await?;
    db::execute(
        &client,
        "CALL app.client_delete($1, $2, $3)",
        &[&actor.id, &id, &q.row_version],
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(search_clients, create_client))
        .routes(routes!(count_clients))
        .routes(routes!(get_client, update_client, delete_client))
}

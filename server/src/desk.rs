//! The service desk (ADR-0007): every active CRM user is an agent and sees every ticket. Each action is one
//! `CALL app.desk_ticket_*` that appends one event to the ticket's stream; the procedures decide whether it is
//! allowed. Dictionaries come from the views `app.v_desk_*`, lists and the ticket from `app.desk_*` functions.
//! The public form (`/api/public/desk`) needs no sign-in and only reads the catalog and names, and reports.

use std::{
    collections::VecDeque,
    sync::Mutex,
    time::{Duration, Instant},
};

use axum::{
    Json,
    body::Body as HttpBody,
    extract::{DefaultBodyLimit, Multipart, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use deadpool_postgres::GenericClient;
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::{
    AppState,
    auth::session::Actor,
    db,
    error::{ApiError, ApiResult},
    page_response,
    shared::{Body, Checks, Params, PathParam, TotalResponse, Validate, cursor, page_size, ts},
};

pub const TAG: &str = "Desk";
pub const PUBLIC_TAG: &str = "Public desk";
/// One request with up to 10 files of 20 MB and the form fields.
const UPLOAD_LIMIT: usize = 210 * 1024 * 1024;
const PRIORITIES: &[&str] = &["CRITICAL", "HIGH", "MEDIUM", "LOW"];

// ---- public form limit ---------------------------------------------------------------------------------------

/// Caps the tickets the public form accepts per minute, for the whole application (one sliding window). The form
/// needs no sign-in, so this is what keeps a script from filling the pool. In memory: each replica counts its own.
pub struct PublicReportLimiter {
    per_minute: usize,
    accepted: Mutex<VecDeque<Instant>>,
}

impl PublicReportLimiter {
    pub fn new(per_minute: usize) -> Self {
        PublicReportLimiter {
            per_minute,
            accepted: Mutex::new(VecDeque::new()),
        }
    }

    pub fn try_acquire(&self) -> bool {
        let now = Instant::now();
        let window = Duration::from_secs(60);
        let mut accepted = self.accepted.lock().unwrap_or_else(|e| e.into_inner());
        while accepted
            .front()
            .is_some_and(|t| now.duration_since(*t) >= window)
        {
            accepted.pop_front();
        }
        if accepted.len() >= self.per_minute {
            return false;
        }
        accepted.push_back(now);
        true
    }
}

// ---- types ---------------------------------------------------------------------------------------------------

/// One row of crm.tickets, the read model of a ticket's stream (ADR-0007). Times are UTC instants.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TicketResponse {
    pub id: i64,
    pub number: String,
    pub title: String,
    pub description: String,
    pub category_key: String,
    pub subcategory_key: String,
    pub priority: String,
    pub original_priority: String,
    #[serde(serialize_with = "ts::instant")]
    pub reported_at: DateTime<Utc>,
    #[serde(serialize_with = "ts::instant")]
    pub deadline: DateTime<Utc>,
    /// IN_POOL, IN_PROGRESS or DONE.
    pub status: String,
    /// In progress: who works on it; resolved: who resolved it.
    pub assignee_user_id: Option<i32>,
    pub assignee_name: Option<String>,
    pub handler_person_id: Option<i32>,
    pub handler_name: Option<String>,
    pub requester_incognito: bool,
    pub requester_person_id: Option<i32>,
    pub requester_name: Option<String>,
    /// AGENT or SELF_SERVICE.
    pub channel: String,
    pub registered_by_name: Option<String>,
    #[serde(serialize_with = "ts::instant")]
    pub recorded_at: DateTime<Utc>,
    #[serde(serialize_with = "ts::instant_opt")]
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_late: bool,
    pub comment_count: i32,
    pub attachment_count: i32,
    pub version: i32,
    #[serde(serialize_with = "ts::instant")]
    pub last_change_at: DateTime<Utc>,
}

page_response!(PageResponseTicketResponse, TicketResponse);

/// One event of a ticket's stream. `data` is the event's JSON; its shape depends on `type`.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TicketEventResponse {
    pub version: i32,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(serialize_with = "ts::instant")]
    pub at: DateTime<Utc>,
    pub actor_user_id: Option<i32>,
    pub actor_name: Option<String>,
    pub data: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TicketCounters {
    pub overdue: i32,
    pub due_soon: i32,
    pub pool: i32,
    pub mine: i32,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentSummary {
    pub id: i32,
    pub login: String,
    pub display_name: String,
    pub role: String,
    pub in_progress: i32,
}

page_response!(PageResponseAgentSummary, AgentSummary);

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PersonResponse {
    pub id: i32,
    pub first_name: String,
    pub last_name: String,
    pub department: String,
    pub email: String,
    pub phone: Option<String>,
}

/// A person as the public form shows them: no contacts.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublicPerson {
    pub id: i32,
    pub first_name: String,
    pub last_name: String,
    pub department: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FieldDefinitionResponse {
    pub key: String,
    /// Polish, like every catalog name; the UI translates it.
    pub label: String,
    /// TEXT, DATE, EMAIL, CHOICE, AMOUNT, IBAN, COUNT, MULTI_CHOICE or TICKET_NUMBER.
    pub kind: String,
    pub placeholder: Option<String>,
    pub required: bool,
    pub options: Vec<String>,
    /// Options the form offers first (they are in `options` too).
    pub favorites: Vec<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubcategoryResponse {
    pub key: String,
    pub name: String,
    pub market: Option<String>,
    pub active: bool,
    /// Report data the subcategory collects, in the form's order (ADR-0010).
    pub fields: Vec<FieldDefinitionResponse>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CategoryResponse {
    pub key: String,
    pub name: String,
    pub department: Option<String>,
    pub subcategories: Vec<SubcategoryResponse>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PriorityResponse {
    pub priority: String,
    pub sla_minutes: i32,
}

/// Everything a form needs to offer: the catalog, the priorities with their SLA and the file extensions an
/// attachment may have (for the file picker's accept attribute).
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeskCatalog {
    pub categories: Vec<CategoryResponse>,
    pub priorities: Vec<PriorityResponse>,
    pub attachment_extensions: Vec<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReportTicketRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub category_key: Option<String>,
    pub subcategory_key: Option<String>,
    pub priority: Option<String>,
    /// When the request was made (a call this morning); at most 30 days back.
    pub reported_at: Option<DateTime<Utc>>,
    pub requester_person_id: Option<i32>,
    pub requester_incognito: Option<bool>,
    /// Report data of the subcategory (ADR-0010), field key -> value; checked by the procedure.
    pub fields: Option<serde_json::Map<String, serde_json::Value>>,
}

impl Validate for ReportTicketRequest {
    fn validate(&self, v: &mut Checks) {
        v.text("title", &self.title, 1, 200);
        v.text("description", &self.description, 1, 4000);
        v.text("categoryKey", &self.category_key, 1, usize::MAX);
        v.text("subcategoryKey", &self.subcategory_key, 1, usize::MAX);
        v.one_of("priority", &self.priority, PRIORITIES);
        v.present("reportedAt", &self.reported_at);
    }
}

/// The public form's fields (a multipart form; the files go in parts named `files`).
#[derive(Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublicReportRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub category_key: Option<String>,
    pub subcategory_key: Option<String>,
    pub priority: Option<String>,
    pub requester_person_id: Option<i32>,
    pub requester_incognito: Option<bool>,
    /// Report data as a JSON object, field key -> value; checked by the procedure.
    pub fields: Option<String>,
}

impl Validate for PublicReportRequest {
    fn validate(&self, v: &mut Checks) {
        v.text("title", &self.title, 1, 200);
        v.text("description", &self.description, 1, 4000);
        v.text("categoryKey", &self.category_key, 1, usize::MAX);
        v.text("subcategoryKey", &self.subcategory_key, 1, usize::MAX);
        v.one_of("priority", &self.priority, PRIORITIES);
        v.max("fields", &self.fields, 20_000);
    }
}

/// What the public form tells the requester: the number to quote and the deadline.
#[derive(Serialize, ToSchema)]
pub struct PublicReportResponse {
    pub number: String,
    #[serde(serialize_with = "ts::instant")]
    pub deadline: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentResponse {
    pub id: Uuid,
    pub file_name: String,
    pub content_type: String,
    pub size: i64,
    pub added_by: String,
    #[serde(serialize_with = "ts::instant")]
    pub at: DateTime<Utc>,
    /// Images and PDF open in the browser; the rest is downloaded.
    pub inline: bool,
}

#[derive(Serialize, ToSchema)]
pub struct TicketFieldResponse {
    pub key: String,
    pub value: String,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateFieldsRequest {
    pub values: Option<serde_json::Map<String, serde_json::Value>>,
}

impl Validate for UpdateFieldsRequest {
    fn validate(&self, v: &mut Checks) {
        v.present("values", &self.values);
    }
}

/// Files of a multipart request (the generated client's model of the attach body).
#[derive(ToSchema)]
#[allow(dead_code)]
pub struct AttachTicketFilesBody {
    #[schema(value_type = Vec<String>, format = Binary)]
    pub files: Vec<Vec<u8>>,
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct DownloadTicketAttachmentParams {
    /// Images and PDF open inline unless `download=true`.
    pub download: Option<bool>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HandOverRequest {
    pub to_user_id: Option<i32>,
    pub reason: Option<String>,
}

impl Validate for HandOverRequest {
    fn validate(&self, v: &mut Checks) {
        v.present("toUserId", &self.to_user_id);
        v.max("reason", &self.reason, 500);
    }
}

#[derive(Deserialize, ToSchema)]
pub struct ReasonRequest {
    pub reason: Option<String>,
}

impl Validate for ReasonRequest {
    fn validate(&self, v: &mut Checks) {
        v.max("reason", &self.reason, 500);
    }
}

#[derive(Deserialize, ToSchema)]
pub struct ChangePriorityRequest {
    pub priority: Option<String>,
    pub reason: Option<String>,
}

impl Validate for ChangePriorityRequest {
    fn validate(&self, v: &mut Checks) {
        v.one_of("priority", &self.priority, PRIORITIES);
        v.max("reason", &self.reason, 500);
    }
}

#[derive(Deserialize, ToSchema)]
pub struct CommentRequest {
    pub text: Option<String>,
}

impl Validate for CommentRequest {
    fn validate(&self, v: &mut Checks) {
        v.text("text", &self.text, 1, 2000);
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssignHandlerRequest {
    /// Null takes the handler off.
    pub person_id: Option<i32>,
    pub reason: Option<String>,
}

impl Validate for AssignHandlerRequest {
    fn validate(&self, v: &mut Checks) {
        v.max("reason", &self.reason, 500);
    }
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct SearchTicketsParams {
    /// POOL, MINE or ALL (default).
    pub scope: Option<String>,
    pub category_key: Option<String>,
    pub subcategory_key: Option<String>,
    pub priority: Option<String>,
    /// OVERDUE or DUE_SOON.
    pub deadline: Option<String>,
    /// OPEN or DONE.
    pub completion: Option<String>,
    pub search: Option<String>,
    pub cursor: Option<String>,
    /// 1-200, default 25.
    pub size: Option<i32>,
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct CountTicketsParams {
    pub scope: Option<String>,
    pub category_key: Option<String>,
    pub subcategory_key: Option<String>,
    pub priority: Option<String>,
    pub deadline: Option<String>,
    pub completion: Option<String>,
    pub search: Option<String>,
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListDeskAgentsParams {
    pub search: Option<String>,
    pub cursor: Option<String>,
    /// 1-200, default 50.
    pub size: Option<i32>,
}

// ---- mapping and repository ----------------------------------------------------------------------------------

pub fn ticket_row(r: &Row) -> TicketResponse {
    TicketResponse {
        id: r.get("id"),
        number: r.get("number"),
        title: r.get("title"),
        description: r.get("description"),
        category_key: r.get("category_key"),
        subcategory_key: r.get("subcategory_key"),
        priority: r.get("priority"),
        original_priority: r.get("original_priority"),
        reported_at: r.get("reported_at"),
        deadline: r.get("deadline"),
        status: r.get("status"),
        assignee_user_id: r.get("assignee_user_id"),
        assignee_name: r.get("assignee_name"),
        handler_person_id: r.get("handler_person_id"),
        handler_name: r.get("handler_name"),
        requester_incognito: r.get("requester_incognito"),
        requester_person_id: r.get("requester_person_id"),
        requester_name: r.get("requester_name"),
        channel: r.get("channel"),
        registered_by_name: r.get("registered_by_name"),
        recorded_at: r.get("recorded_at"),
        resolved_at: r.get("resolved_at"),
        resolved_late: r.get("resolved_late"),
        comment_count: r.get("comment_count"),
        attachment_count: r.get("attachment_count"),
        version: r.get("version"),
        last_change_at: r.get("last_change_at"),
    }
}

pub async fn get<C: GenericClient + Sync>(
    client: &C,
    actor_id: i32,
    id: i64,
) -> ApiResult<TicketResponse> {
    let row = db::query_one(
        client,
        "SELECT * FROM app.desk_ticket_get($1, $2)",
        &[&actor_id, &id],
    )
    .await?;
    Ok(ticket_row(&row))
}

/// One action of an agent: the procedure, then the ticket as it is now, in one transaction.
async fn act(
    state: &AppState,
    actor_id: i32,
    id: i64,
    sql: &str,
    params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
) -> ApiResult<Json<TicketResponse>> {
    let mut client = state.db.get().await?;
    let tx = client.transaction().await?;
    db::execute(&tx, sql, params).await?;
    let ticket = get(&tx, actor_id, id).await?;
    tx.commit().await?;
    Ok(Json(ticket))
}

pub(crate) async fn catalog(state: &AppState) -> ApiResult<DeskCatalog> {
    let client = state.db.get().await?;
    let rows = db::query(
        &client,
        "SELECT category_key, category_name, department, subcategory_key, subcategory_name, market, is_active \
         FROM app.v_desk_catalog ORDER BY category_order, subcategory_order",
        &[],
    )
    .await?;
    let options = db::query(
        &client,
        "SELECT field_key, option_value, is_favorite FROM app.v_desk_field_options ORDER BY field_key, sort_order",
        &[],
    )
    .await?;
    let fields = db::query(
        &client,
        "SELECT subcategory_key, field_key, label, kind, placeholder, is_required FROM app.v_desk_fields \
         ORDER BY subcategory_key, sort_order",
        &[],
    )
    .await?;
    let fields_of = |subcategory: &str| -> Vec<FieldDefinitionResponse> {
        fields
            .iter()
            .filter(|f| f.get::<_, &str>("subcategory_key") == subcategory)
            .map(|f| {
                let key: String = f.get("field_key");
                let of_field = || {
                    options
                        .iter()
                        .filter(|o| o.get::<_, &str>("field_key") == key)
                };
                FieldDefinitionResponse {
                    label: f.get("label"),
                    kind: f.get("kind"),
                    placeholder: f.get("placeholder"),
                    required: f.get("is_required"),
                    options: of_field().map(|o| o.get("option_value")).collect(),
                    favorites: of_field()
                        .filter(|o| o.get::<_, bool>("is_favorite"))
                        .map(|o| o.get("option_value"))
                        .collect(),
                    key,
                }
            })
            .collect()
    };
    let mut categories: Vec<CategoryResponse> = Vec::new();
    for r in &rows {
        let key: String = r.get("category_key");
        if categories.last().is_none_or(|c| c.key != key) {
            categories.push(CategoryResponse {
                key,
                name: r.get("category_name"),
                department: r.get("department"),
                subcategories: Vec::new(),
            });
        }
        if let Some(category) = categories.last_mut() {
            let key: String = r.get("subcategory_key");
            category.subcategories.push(SubcategoryResponse {
                fields: fields_of(&key),
                key,
                name: r.get("subcategory_name"),
                market: r.get("market"),
                active: r.get("is_active"),
            });
        }
    }
    let priorities = db::query(
        &client,
        "SELECT priority, sla_minutes FROM app.v_desk_priorities ORDER BY sort_order",
        &[],
    )
    .await?
    .iter()
    .map(|r| PriorityResponse {
        priority: r.get("priority"),
        sla_minutes: r.get("sla_minutes"),
    })
    .collect();
    let attachment_extensions = db::query(
        &client,
        "SELECT extension FROM app.v_desk_attachment_types ORDER BY extension",
        &[],
    )
    .await?
    .iter()
    .map(|r| r.get("extension"))
    .collect();
    Ok(DeskCatalog {
        categories,
        priorities,
        attachment_extensions,
    })
}

fn invalid_cursor() -> ApiError {
    ApiError::bad_request("Invalid cursor.")
}

// ---- endpoints for agents ------------------------------------------------------------------------------------

/// Categories, subcategories and priorities with their SLA.
#[utoipa::path(get, path = "/api/desk/catalog", tag = TAG, operation_id = "getDeskCatalog",
    responses((status = 200, body = DeskCatalog)))]
async fn get_desk_catalog(
    State(state): State<AppState>,
    _actor: Actor,
) -> ApiResult<Json<DeskCatalog>> {
    Ok(Json(catalog(&state).await?))
}

/// Tickets page. scope POOL / MINE / ALL; open tickets by deadline first, then resolved ones, newest first.
#[utoipa::path(get, path = "/api/desk/tickets", tag = TAG, operation_id = "searchTickets",
    params(SearchTicketsParams), responses((status = 200, body = PageResponseTicketResponse)))]
async fn search_tickets(
    State(state): State<AppState>,
    Actor(actor): Actor,
    Params(q): Params<SearchTicketsParams>,
) -> ApiResult<Json<PageResponseTicketResponse>> {
    let size = page_size(Some(q.size.unwrap_or(25)))?;
    let (after_phase, after_time, after_id): (Option<i32>, Option<DateTime<Utc>>, Option<i64>) =
        match cursor::decode(q.cursor.as_deref(), 3)? {
            Some(p) => (
                Some(p[0].parse().map_err(|_| invalid_cursor())?),
                Some(
                    DateTime::parse_from_rfc3339(&p[1])
                        .map_err(|_| invalid_cursor())?
                        .with_timezone(&Utc),
                ),
                Some(p[2].parse().map_err(|_| invalid_cursor())?),
            ),
            None => (None, None, None),
        };
    let scope = q.scope.unwrap_or_else(|| "ALL".into());
    let client = state.db.get().await?;
    let rows = db::query(
        &client,
        "SELECT * FROM app.desk_ticket_search($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        &[
            &actor.id,
            &scope,
            &q.category_key,
            &q.subcategory_key,
            &q.priority,
            &q.deadline,
            &q.completion,
            &q.search,
            &after_phase,
            &after_time,
            &after_id,
            &size,
        ],
    )
    .await?;
    let items = rows.iter().map(ticket_row).collect();
    Ok(Json(PageResponseTicketResponse::new(cursor::page(
        items,
        size,
        |t| match t.resolved_at.filter(|_| t.status == "DONE") {
            Some(resolved) => cursor::encode(&[&1, &resolved.to_rfc3339(), &t.id]),
            None => cursor::encode(&[&0, &t.deadline.to_rfc3339(), &t.id]),
        },
    ))))
}

/// Number of tickets matching the same filters as the list. Call it only on request.
#[utoipa::path(get, path = "/api/desk/tickets/count", tag = TAG, operation_id = "countTickets",
    params(CountTicketsParams), responses((status = 200, body = TotalResponse)))]
async fn count_tickets(
    State(state): State<AppState>,
    Actor(actor): Actor,
    Params(q): Params<CountTicketsParams>,
) -> ApiResult<Json<TotalResponse>> {
    let scope = q.scope.unwrap_or_else(|| "ALL".into());
    let client = state.db.get().await?;
    let row = db::query_one(
        &client,
        "SELECT total FROM app.desk_ticket_count($1, $2, $3, $4, $5, $6, $7, $8)",
        &[
            &actor.id,
            &scope,
            &q.category_key,
            &q.subcategory_key,
            &q.priority,
            &q.deadline,
            &q.completion,
            &q.search,
        ],
    )
    .await?;
    Ok(Json(TotalResponse {
        total: row.get("total"),
    }))
}

/// Counters of the top bar: overdue, due soon, in the pool, mine.
#[utoipa::path(get, path = "/api/desk/tickets/counters", tag = TAG, operation_id = "getTicketCounters",
    responses((status = 200, body = TicketCounters)))]
async fn get_ticket_counters(
    State(state): State<AppState>,
    Actor(actor): Actor,
) -> ApiResult<Json<TicketCounters>> {
    let client = state.db.get().await?;
    let r = db::query_one(
        &client,
        "SELECT overdue, due_soon, pool, mine FROM app.desk_ticket_counters($1)",
        &[&actor.id],
    )
    .await?;
    Ok(Json(TicketCounters {
        overdue: r.get("overdue"),
        due_soon: r.get("due_soon"),
        pool: r.get("pool"),
        mine: r.get("mine"),
    }))
}

/// Register a ticket for a requester (a call, an e-mail); may be dated back up to 30 days.
#[utoipa::path(post, path = "/api/desk/tickets", tag = TAG, operation_id = "reportTicket",
    request_body = ReportTicketRequest, responses((status = 201, body = TicketResponse)))]
async fn report_ticket(
    State(state): State<AppState>,
    Actor(actor): Actor,
    Body(b): Body<ReportTicketRequest>,
) -> ApiResult<(StatusCode, Json<TicketResponse>)> {
    let mut client = state.db.get().await?;
    let tx = client.transaction().await?;
    let row = db::query_one(
        &tx,
        "CALL app.desk_ticket_report($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NULL)",
        &[
            &actor.id,
            &b.title,
            &b.description,
            &b.category_key,
            &b.subcategory_key,
            &b.priority,
            &b.reported_at,
            &b.requester_person_id,
            &b.requester_incognito,
            &b.fields.map(serde_json::Value::Object),
        ],
    )
    .await?;
    let ticket = get(&tx, actor.id, row.get("p_ticket_id")).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(ticket)))
}

/// Ticket details.
#[utoipa::path(get, path = "/api/desk/tickets/{id}", tag = TAG, operation_id = "getTicket",
    params(("id" = i64, Path)), responses((status = 200, body = TicketResponse)))]
async fn get_ticket(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
) -> ApiResult<Json<TicketResponse>> {
    let client = state.db.get().await?;
    Ok(Json(get(&client, actor.id, id).await?))
}

/// Ticket by its public number (HAL-XXXXXXXX).
#[utoipa::path(get, path = "/api/desk/tickets/by-number/{number}", tag = TAG, operation_id = "findTicket",
    params(("number" = String, Path)), responses((status = 200, body = TicketResponse)))]
async fn find_ticket(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(number): PathParam<String>,
) -> ApiResult<Json<TicketResponse>> {
    let client = state.db.get().await?;
    let row = db::query_one(
        &client,
        "SELECT * FROM app.desk_ticket_find($1, $2)",
        &[&actor.id, &number],
    )
    .await?;
    Ok(Json(ticket_row(&row)))
}

/// The ticket's events, oldest first: its timeline.
#[utoipa::path(get, path = "/api/desk/tickets/{id}/events", tag = TAG, operation_id = "listTicketEvents",
    params(("id" = i64, Path)), responses((status = 200, body = Vec<TicketEventResponse>)))]
async fn list_ticket_events(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
) -> ApiResult<Json<Vec<TicketEventResponse>>> {
    let client = state.db.get().await?;
    let rows = db::query(
        &client,
        "SELECT * FROM app.desk_ticket_event_list($1, $2)",
        &[&actor.id, &id],
    )
    .await?;
    Ok(Json(
        rows.iter()
            .map(|r| TicketEventResponse {
                version: r.get("version"),
                kind: r.get("event_type"),
                at: r.get("at"),
                actor_user_id: r.get("actor_user_id"),
                actor_name: r.get("actor_name"),
                data: r.get("data"),
            })
            .collect(),
    ))
}

/// Take a ticket from the pool. 409 when someone was faster.
#[utoipa::path(post, path = "/api/desk/tickets/{id}/take", tag = TAG, operation_id = "takeTicket",
    params(("id" = i64, Path)), responses((status = 200, body = TicketResponse)))]
async fn take_ticket(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
) -> ApiResult<Json<TicketResponse>> {
    act(
        &state,
        actor.id,
        id,
        "CALL app.desk_ticket_take($1, $2)",
        &[&actor.id, &id],
    )
    .await
}

/// Hand the ticket over to another agent (only the agent working on it).
#[utoipa::path(post, path = "/api/desk/tickets/{id}/hand-over", tag = TAG, operation_id = "handOverTicket",
    params(("id" = i64, Path)), request_body = HandOverRequest, responses((status = 200, body = TicketResponse)))]
async fn hand_over_ticket(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
    Body(b): Body<HandOverRequest>,
) -> ApiResult<Json<TicketResponse>> {
    act(
        &state,
        actor.id,
        id,
        "CALL app.desk_ticket_hand_over($1, $2, $3, $4)",
        &[&actor.id, &id, &b.to_user_id, &b.reason],
    )
    .await
}

/// Give the ticket back to the pool (only the agent working on it).
#[utoipa::path(post, path = "/api/desk/tickets/{id}/return", tag = TAG, operation_id = "returnTicket",
    params(("id" = i64, Path)), request_body = ReasonRequest, responses((status = 200, body = TicketResponse)))]
async fn return_ticket(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
    Body(b): Body<ReasonRequest>,
) -> ApiResult<Json<TicketResponse>> {
    act(
        &state,
        actor.id,
        id,
        "CALL app.desk_ticket_return($1, $2, $3)",
        &[&actor.id, &id, &b.reason],
    )
    .await
}

/// Change the priority; the deadline is recomputed from the report date.
#[utoipa::path(post, path = "/api/desk/tickets/{id}/priority", tag = TAG, operation_id = "changeTicketPriority",
    params(("id" = i64, Path)), request_body = ChangePriorityRequest, responses((status = 200, body = TicketResponse)))]
async fn change_ticket_priority(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
    Body(b): Body<ChangePriorityRequest>,
) -> ApiResult<Json<TicketResponse>> {
    act(
        &state,
        actor.id,
        id,
        "CALL app.desk_ticket_change_priority($1, $2, $3, $4)",
        &[&actor.id, &id, &b.priority, &b.reason],
    )
    .await
}

/// Resolve the ticket (only the agent working on it).
#[utoipa::path(post, path = "/api/desk/tickets/{id}/resolve", tag = TAG, operation_id = "resolveTicket",
    params(("id" = i64, Path)), responses((status = 200, body = TicketResponse)))]
async fn resolve_ticket(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
) -> ApiResult<Json<TicketResponse>> {
    act(
        &state,
        actor.id,
        id,
        "CALL app.desk_ticket_resolve($1, $2)",
        &[&actor.id, &id],
    )
    .await
}

/// Add a comment (any state).
#[utoipa::path(post, path = "/api/desk/tickets/{id}/comments", tag = TAG, operation_id = "commentTicket",
    params(("id" = i64, Path)), request_body = CommentRequest, responses((status = 200, body = TicketResponse)))]
async fn comment_ticket(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
    Body(b): Body<CommentRequest>,
) -> ApiResult<Json<TicketResponse>> {
    act(
        &state,
        actor.id,
        id,
        "CALL app.desk_ticket_comment($1, $2, $3)",
        &[&actor.id, &id, &b.text],
    )
    .await
}

/// Name the person who does the work, or take them off (only the agent working on it).
#[utoipa::path(post, path = "/api/desk/tickets/{id}/handler", tag = TAG, operation_id = "assignTicketHandler",
    params(("id" = i64, Path)), request_body = AssignHandlerRequest, responses((status = 200, body = TicketResponse)))]
async fn assign_ticket_handler(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
    Body(b): Body<AssignHandlerRequest>,
) -> ApiResult<Json<TicketResponse>> {
    act(
        &state,
        actor.id,
        id,
        "CALL app.desk_ticket_assign_handler($1, $2, $3, $4)",
        &[&actor.id, &id, &b.person_id, &b.reason],
    )
    .await
}

/// Agents (active CRM users) with their tickets in progress; by name, `search` = name prefix.
#[utoipa::path(get, path = "/api/desk/agents", tag = TAG, operation_id = "listDeskAgents",
    params(ListDeskAgentsParams), responses((status = 200, body = PageResponseAgentSummary)))]
async fn list_desk_agents(
    State(state): State<AppState>,
    Actor(actor): Actor,
    Params(q): Params<ListDeskAgentsParams>,
) -> ApiResult<Json<PageResponseAgentSummary>> {
    let size = page_size(q.size)?;
    let parts = cursor::decode(q.cursor.as_deref(), 2)?;
    let after_id: Option<i32> = match &parts {
        Some(p) => Some(p[0].parse().map_err(|_| invalid_cursor())?),
        None => None,
    };
    let after_name: Option<String> = parts.map(|p| p[1].clone());
    let client = state.db.get().await?;
    let rows = db::query(
        &client,
        "SELECT * FROM app.desk_agent_list($1, $2, $3, $4, $5)",
        &[&actor.id, &q.search, &after_name, &after_id, &size],
    )
    .await?;
    let agents = rows
        .iter()
        .map(|r| AgentSummary {
            id: r.get("id"),
            login: r.get("login"),
            display_name: r.get("display_name"),
            role: r.get("role"),
            in_progress: r.get("in_progress"),
        })
        .collect();
    Ok(Json(PageResponseAgentSummary::new(cursor::page(
        agents,
        size,
        |a| cursor::encode(&[&a.id, &a.display_name]),
    ))))
}

/// People (requesters and handlers) with their contacts.
#[utoipa::path(get, path = "/api/desk/people", tag = TAG, operation_id = "listDeskPeople",
    responses((status = 200, body = Vec<PersonResponse>)))]
async fn list_desk_people(
    State(state): State<AppState>,
    Actor(actor): Actor,
) -> ApiResult<Json<Vec<PersonResponse>>> {
    let client = state.db.get().await?;
    let rows = db::query(
        &client,
        "SELECT * FROM app.desk_person_list($1)",
        &[&actor.id],
    )
    .await?;
    Ok(Json(
        rows.iter()
            .map(|r| PersonResponse {
                id: r.get("id"),
                first_name: r.get("first_name"),
                last_name: r.get("last_name"),
                department: r.get("department"),
                email: r.get("email"),
                phone: r.get("phone"),
            })
            .collect(),
    ))
}

// ---- the public form -----------------------------------------------------------------------------------------

/// Categories, subcategories and priorities for the public form.
#[utoipa::path(get, path = "/api/public/desk/catalog", tag = PUBLIC_TAG, operation_id = "getPublicDeskCatalog",
    responses((status = 200, body = DeskCatalog)))]
async fn get_public_desk_catalog(State(state): State<AppState>) -> ApiResult<Json<DeskCatalog>> {
    Ok(Json(catalog(&state).await?))
}

/// People to pick the requester from: names and departments only.
#[utoipa::path(get, path = "/api/public/desk/people", tag = PUBLIC_TAG, operation_id = "listPublicPeople",
    responses((status = 200, body = Vec<PublicPerson>)))]
async fn list_public_people(State(state): State<AppState>) -> ApiResult<Json<Vec<PublicPerson>>> {
    let client = state.db.get().await?;
    let rows = db::query(
        &client,
        "SELECT id, first_name, last_name, department FROM app.v_desk_public_people \
         ORDER BY last_name, first_name, id",
        &[],
    )
    .await?;
    Ok(Json(
        rows.iter()
            .map(|r| PublicPerson {
                id: r.get("id"),
                first_name: r.get("first_name"),
                last_name: r.get("last_name"),
                department: r.get("department"),
            })
            .collect(),
    ))
}

/// A file of a multipart request, read into memory (20 MB at most each; the procedure checks the size).
pub struct Upload {
    pub file_name: String,
    pub bytes: Bytes,
}

fn bad_multipart(e: axum::extract::multipart::MultipartError) -> ApiError {
    if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
        return ApiError::Problem {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            title: "Payload too large",
            detail: "The request is too large: at most 10 files of 20 MB.".into(),
            error_number: None,
        };
    }
    ApiError::bad_request("Failed to read request.")
}

/// Every part named `files`; a part without a name and without bytes (an empty file input) is skipped.
async fn read_files(mut form: Multipart) -> ApiResult<Vec<Upload>> {
    let mut files = Vec::new();
    while let Some(field) = form.next_field().await.map_err(bad_multipart)? {
        if field.name() != Some("files") {
            continue;
        }
        let file_name = field.file_name().unwrap_or_default().to_owned();
        let bytes = field.bytes().await.map_err(bad_multipart)?;
        if file_name.trim().is_empty() && bytes.is_empty() {
            continue;
        }
        files.push(Upload { file_name, bytes });
    }
    Ok(files)
}

/// The fields and the files of the public form.
async fn read_public_form(mut form: Multipart) -> ApiResult<(PublicReportRequest, Vec<Upload>)> {
    let bad = bad_multipart;
    let mut r = PublicReportRequest::default();
    let mut files = Vec::new();
    while let Some(field) = form.next_field().await.map_err(bad)? {
        let name = field.name().unwrap_or_default().to_owned();
        if name == "files" {
            let file_name = field.file_name().unwrap_or_default().to_owned();
            let bytes = field.bytes().await.map_err(bad)?;
            if !(file_name.trim().is_empty() && bytes.is_empty()) {
                files.push(Upload { file_name, bytes });
            }
            continue;
        }
        let value = field.text().await.map_err(bad)?;
        let text = Some(value.clone()).filter(|v| !v.is_empty());
        match name.as_str() {
            "title" => r.title = text,
            "description" => r.description = text,
            "categoryKey" => r.category_key = text,
            "subcategoryKey" => r.subcategory_key = text,
            "priority" => r.priority = text,
            "fields" => r.fields = text,
            "requesterPersonId" => {
                r.requester_person_id = match text {
                    Some(v) => Some(v.trim().parse().map_err(|_| {
                        ApiError::bad_request("requesterPersonId: must be a number")
                    })?),
                    None => None,
                }
            }
            "requesterIncognito" => {
                r.requester_incognito = match text.as_deref().map(str::trim) {
                    Some("true") => Some(true),
                    Some("false") => Some(false),
                    None => None,
                    Some(_) => {
                        return Err(ApiError::bad_request(
                            "requesterIncognito: must be true or false",
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    let mut checks = Checks::default();
    r.validate(&mut checks);
    checks.into_result()?;
    Ok((r, files))
}

// ---- attachments: check, store, record (ADR-0009) -------------------------------------------------------------

fn storage_unavailable(error: anyhow::Error) -> ApiError {
    tracing::error!(
        error = format!("{error:#}"),
        "attachment storage call failed"
    );
    ApiError::Problem {
        status: StatusCode::SERVICE_UNAVAILABLE,
        title: "Storage unavailable",
        detail: "File storage is not available.".into(),
        error_number: None,
    }
}

/// The order of an upload: the procedure checks each file (name, type by extension, size) before any byte is
/// stored, the bytes go to the bucket, then `record` writes the events in one transaction - checking the rules again
/// under the ticket's lock. When the database refuses (or the bucket fails midway), the stored bytes are deleted
/// again: a crash between the steps leaves an orphan object, never an event pointing at nothing.
async fn with_stored<T, F, Fut>(state: &AppState, files: Vec<Upload>, record: F) -> ApiResult<T>
where
    F: FnOnce(Option<serde_json::Value>) -> Fut,
    Fut: std::future::Future<Output = ApiResult<T>>,
{
    if files.is_empty() {
        return record(None).await;
    }
    let mut checked = Vec::with_capacity(files.len());
    {
        let client = state.db.get().await?;
        for file in files {
            let row = db::query_one(
                &client,
                "SELECT file_name, content_type FROM app.desk_attachment_check($1, $2)",
                &[&file.file_name, &(file.bytes.len() as i64)],
            )
            .await?;
            let content_type: String = row.get("content_type");
            checked.push((file, content_type));
        }
    }
    let mut keys = Vec::new();
    let mut stored = Vec::new();
    for (file, content_type) in checked {
        let id = Uuid::new_v4();
        let key = format!("attachments/{id}");
        let size = file.bytes.len() as i64;
        if let Err(e) = state.storage.put(&key, file.bytes, &content_type).await {
            state.storage.delete(&keys).await;
            return Err(storage_unavailable(e));
        }
        stored.push(serde_json::json!({"id": id, "fileName": file.file_name, "size": size, "storageKey": key}));
        keys.push(key);
    }
    match record(Some(serde_json::Value::Array(stored))).await {
        Ok(value) => Ok(value),
        Err(error) => {
            state.storage.delete(&keys).await;
            Err(error)
        }
    }
}

/// `inline; filename="..."; filename*=UTF-8''...` - the ASCII form for old clients, the exact name in UTF-8.
pub(crate) fn content_disposition(kind: &str, file_name: &str) -> String {
    let ascii: String = file_name
        .chars()
        .map(|c| {
            if c.is_ascii_graphic() && c != '"' && c != '\\' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let encoded: String = file_name
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'!'
            | b'#'
            | b'$'
            | b'&'
            | b'+'
            | b'-'
            | b'.'
            | b'^'
            | b'_'
            | b'`'
            | b'|'
            | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect();
    format!("{kind}; filename=\"{ascii}\"; filename*=UTF-8''{encoded}")
}

/// Report a ticket (reported now, into the pool). 429 when too many were sent this minute.
#[utoipa::path(post, path = "/api/public/desk/tickets", tag = PUBLIC_TAG, operation_id = "reportPublicTicket",
    request_body(content = PublicReportRequest, content_type = "multipart/form-data"),
    responses((status = 201, body = PublicReportResponse)))]
async fn report_public_ticket(
    State(state): State<AppState>,
    form: Multipart,
) -> ApiResult<(StatusCode, Json<PublicReportResponse>)> {
    if !state.public_reports.try_acquire() {
        return Err(ApiError::Problem {
            status: StatusCode::TOO_MANY_REQUESTS,
            title: "Too many requests",
            detail: "Too many tickets were sent in the last minute. Try again shortly.".into(),
            error_number: None,
        });
    }
    let (b, files) = read_public_form(form).await?;
    let fields: Option<serde_json::Value> = match b.fields.as_deref() {
        None => None,
        Some(text) => match serde_json::from_str(text) {
            Ok(v @ serde_json::Value::Object(_)) => Some(v),
            _ => return Err(ApiError::bad_request("fields: must be a JSON object")),
        },
    };
    let db_state = state.clone();
    let response = with_stored(&state, files, |attachments| async move {
        let client = db_state.db.get().await?;
        let row = db::query_one(
            &client,
            "CALL app.desk_ticket_report_public($1, $2, $3, $4, $5, $6, $7, $8, $9, NULL, NULL)",
            &[
                &b.title,
                &b.description,
                &b.category_key,
                &b.subcategory_key,
                &b.priority,
                &b.requester_person_id,
                &b.requester_incognito,
                &fields,
                &attachments,
            ],
        )
        .await?;
        Ok(PublicReportResponse {
            number: row.get("p_number"),
            deadline: row.get("p_deadline"),
        })
    })
    .await?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// The ticket's attachments, oldest first.
#[utoipa::path(get, path = "/api/desk/tickets/{id}/attachments", tag = TAG, operation_id = "listTicketAttachments",
    params(("id" = i64, Path)), responses((status = 200, body = Vec<AttachmentResponse>)))]
async fn list_ticket_attachments(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
) -> ApiResult<Json<Vec<AttachmentResponse>>> {
    let client = state.db.get().await?;
    let rows = db::query(
        &client,
        "SELECT * FROM app.desk_attachment_list($1, $2)",
        &[&actor.id, &id],
    )
    .await?;
    Ok(Json(
        rows.iter()
            .map(|r| AttachmentResponse {
                id: r.get("attachment_id"),
                file_name: r.get("file_name"),
                content_type: r.get("content_type"),
                size: r.get("size_bytes"),
                added_by: r.get("added_by"),
                at: r.get("at"),
                inline: r.get("is_inline"),
            })
            .collect(),
    ))
}

/// Attach files (any state): at most 10 per ticket, 20 MB each, images, PDF, Office, TXT/CSV, ZIP, MP4/MOV - never
/// SVG or HTML.
#[utoipa::path(post, path = "/api/desk/tickets/{id}/attachments", tag = TAG, operation_id = "attachTicketFiles",
    params(("id" = i64, Path)), request_body(content = AttachTicketFilesBody, content_type = "multipart/form-data"),
    responses((status = 200, body = TicketResponse)))]
async fn attach_ticket_files(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
    form: Multipart,
) -> ApiResult<Json<TicketResponse>> {
    let files = read_files(form).await?;
    if files.is_empty() {
        return Err(ApiError::bad_request("files: choose at least one file"));
    }
    let db_state = state.clone();
    let ticket = with_stored(&state, files, |attachments| async move {
        let mut client = db_state.db.get().await?;
        let tx = client.transaction().await?;
        db::execute(
            &tx,
            "CALL app.desk_ticket_attach($1, $2, $3)",
            &[&actor.id, &id, &attachments],
        )
        .await?;
        let ticket = get(&tx, actor.id, id).await?;
        tx.commit().await?;
        Ok(ticket)
    })
    .await?;
    Ok(Json(ticket))
}

/// Download an attachment. Images and PDF inline (or with download=true as a file), the rest as a file; always
/// nosniff and a sandbox CSP.
#[utoipa::path(get, path = "/api/desk/tickets/{id}/attachments/{attachmentId}", tag = TAG,
    operation_id = "downloadTicketAttachment",
    params(("id" = i64, Path), ("attachmentId" = Uuid, Path), DownloadTicketAttachmentParams),
    responses((status = 200, description = "The file", content_type = "application/octet-stream")))]
async fn download_ticket_attachment(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam((id, attachment_id)): PathParam<(i64, Uuid)>,
    Params(q): Params<DownloadTicketAttachmentParams>,
) -> ApiResult<Response> {
    let row = {
        let client = state.db.get().await?;
        db::query_one(
            &client,
            "SELECT * FROM app.desk_attachment_get($1, $2, $3)",
            &[&actor.id, &id, &attachment_id],
        )
        .await?
    };
    let file_name: String = row.get("file_name");
    let content_type: String = row.get("content_type");
    let size: i64 = row.get("size_bytes");
    let inline = row.get::<_, bool>("is_inline") && !q.download.unwrap_or(false);
    let key: String = row.get("storage_key");
    let body = state
        .storage
        .open(&key)
        .await
        .map_err(storage_unavailable)?;
    let stream = tokio_util::io::ReaderStream::new(body.into_async_read());
    let disposition = content_disposition(if inline { "inline" } else { "attachment" }, &file_name);
    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CONTENT_LENGTH, size.to_string()),
            (header::CONTENT_DISPOSITION, disposition),
            // The type is the one the extension allows; the browser must not guess another, nor run anything.
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_owned()),
            (header::CONTENT_SECURITY_POLICY, "sandbox".to_owned()),
            (header::CACHE_CONTROL, "private, max-age=3600".to_owned()),
        ],
        HttpBody::from_stream(stream),
    )
        .into_response())
}

/// The ticket's report data: current values in the subcategory's order.
#[utoipa::path(get, path = "/api/desk/tickets/{id}/fields", tag = TAG, operation_id = "listTicketFields",
    params(("id" = i64, Path)), responses((status = 200, body = Vec<TicketFieldResponse>)))]
async fn list_ticket_fields(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
) -> ApiResult<Json<Vec<TicketFieldResponse>>> {
    let client = state.db.get().await?;
    let rows = db::query(
        &client,
        "SELECT field_key, value FROM app.desk_ticket_field_list($1, $2)",
        &[&actor.id, &id],
    )
    .await?;
    Ok(Json(
        rows.iter()
            .map(|r| TicketFieldResponse {
                key: r.get("field_key"),
                value: r.get("value"),
            })
            .collect(),
    ))
}

/// Fill in or correct the report data (any agent, any state). `values` is the whole new set; an empty value clears
/// a field.
#[utoipa::path(put, path = "/api/desk/tickets/{id}/fields", tag = TAG, operation_id = "updateTicketFields",
    params(("id" = i64, Path)), request_body = UpdateFieldsRequest, responses((status = 200, body = TicketResponse)))]
async fn update_ticket_fields(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
    Body(b): Body<UpdateFieldsRequest>,
) -> ApiResult<Json<TicketResponse>> {
    let values = b.values.map(serde_json::Value::Object);
    act(
        &state,
        actor.id,
        id,
        "CALL app.desk_ticket_update_fields($1, $2, $3)",
        &[&actor.id, &id, &values],
    )
    .await
}

/// Marketing campaigns ordered for this ticket (their related-ticket is its number).
#[utoipa::path(get, path = "/api/desk/tickets/{id}/linked", tag = TAG, operation_id = "listLinkedTickets",
    params(("id" = i64, Path)), responses((status = 200, body = Vec<TicketResponse>)))]
async fn list_linked_tickets(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
) -> ApiResult<Json<Vec<TicketResponse>>> {
    let client = state.db.get().await?;
    let rows = db::query(
        &client,
        "SELECT * FROM app.desk_ticket_linked_list($1, $2)",
        &[&actor.id, &id],
    )
    .await?;
    Ok(Json(rows.iter().map(ticket_row).collect()))
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_desk_catalog))
        .routes(routes!(search_tickets, report_ticket))
        .routes(routes!(count_tickets))
        .routes(routes!(get_ticket_counters))
        .routes(routes!(get_ticket))
        .routes(routes!(find_ticket))
        .routes(routes!(list_ticket_events))
        .routes(routes!(take_ticket))
        .routes(routes!(hand_over_ticket))
        .routes(routes!(return_ticket))
        .routes(routes!(change_ticket_priority))
        .routes(routes!(resolve_ticket))
        .routes(routes!(comment_ticket))
        .routes(routes!(assign_ticket_handler))
        .routes(routes!(list_desk_agents))
        .routes(routes!(list_desk_people))
        .routes(routes!(get_public_desk_catalog))
        .routes(routes!(list_public_people))
        .routes(routes!(download_ticket_attachment))
        .routes(routes!(list_ticket_fields, update_ticket_fields))
        .routes(routes!(list_linked_tickets))
        .merge(
            OpenApiRouter::new()
                .routes(routes!(list_ticket_attachments, attach_ticket_files))
                .routes(routes!(report_public_ticket))
                .layer(DefaultBodyLimit::max(UPLOAD_LIMIT)),
        )
}

#[cfg(test)]
mod tests {
    use super::PublicReportLimiter;

    #[test]
    fn the_limit_is_per_minute() {
        let limiter = PublicReportLimiter::new(2);
        assert!(limiter.try_acquire());
        assert!(limiter.try_acquire());
        assert!(!limiter.try_acquire());
    }
}

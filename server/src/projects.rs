//! Projects, their stages and comments (ADR-0005). Writes are `CALL app.project_*`; every project is read from
//! `crm.v_projects` through `app.project_search`, `app.project_get` and `app.project_count`.

use axum::{Json, extract::State, http::StatusCode};
use chrono::{DateTime, NaiveDate, Utc};
use deadpool_postgres::GenericClient;
use rust_decimal::Decimal;
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

pub const TAG: &str = "Projects";
const STAGES: &[&str] = &["NEW", "ACCEPTED", "ACTIVE", "ON_HOLD", "CLOSED", "DECLINED"];

/// One row of crm.v_projects.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectResponse {
    pub id: i64,
    pub name: String,
    pub stage: String,
    pub start_date: NaiveDate,
    pub due_date: NaiveDate,
    pub budget: Decimal,
    pub overdue: bool,
    #[serde(serialize_with = "ts::local")]
    pub stage_changed_at: DateTime<Utc>,
    pub client_id: i64,
    pub client_name: String,
    pub client_city: Option<String>,
    pub team_id: i32,
    pub team_name: String,
    pub owner_user_id: i32,
    pub owner_login: String,
    pub owner_display_name: String,
    #[serde(serialize_with = "ts::local")]
    pub updated_at: DateTime<Utc>,
    pub row_version: String,
}

page_response!(PageResponseProjectResponse, ProjectResponse);

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StageHistoryEntry {
    pub id: i64,
    pub from_stage: Option<String>,
    pub to_stage: String,
    #[serde(serialize_with = "ts::local")]
    pub changed_at: DateTime<Utc>,
    pub changed_by: Option<i32>,
    pub changed_by_login: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommentResponse {
    pub id: i64,
    pub project_id: i64,
    pub author_id: i32,
    pub author_login: String,
    pub author_display_name: String,
    pub body: String,
    #[serde(serialize_with = "ts::local")]
    pub created_at: DateTime<Utc>,
}

page_response!(PageResponseCommentResponse, CommentResponse);

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub client_id: Option<i64>,
    pub team_id: Option<i32>,
    pub owner_user_id: Option<i32>,
    pub name: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub budget: Option<Decimal>,
}

impl Validate for CreateProjectRequest {
    fn validate(&self, v: &mut Checks) {
        v.present("clientId", &self.client_id);
        v.present("teamId", &self.team_id);
        v.present("ownerUserId", &self.owner_user_id);
        v.text("name", &self.name, 1, 200);
        v.present("startDate", &self.start_date);
        v.present("dueDate", &self.due_date);
        v.money("budget", &self.budget);
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    pub team_id: Option<i32>,
    pub owner_user_id: Option<i32>,
    pub name: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub budget: Option<Decimal>,
    pub row_version: Option<String>,
}

impl Validate for UpdateProjectRequest {
    fn validate(&self, v: &mut Checks) {
        v.present("teamId", &self.team_id);
        v.present("ownerUserId", &self.owner_user_id);
        v.text("name", &self.name, 1, 200);
        v.present("startDate", &self.start_date);
        v.present("dueDate", &self.due_date);
        v.money("budget", &self.budget);
        v.row_version(&self.row_version, true);
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangeStageRequest {
    pub stage: Option<String>,
    pub comment: Option<String>,
    pub row_version: Option<String>,
}

impl Validate for ChangeStageRequest {
    fn validate(&self, v: &mut Checks) {
        v.one_of("stage", &self.stage, STAGES);
        v.max("comment", &self.comment, 4000);
        v.row_version(&self.row_version, true);
    }
}

#[derive(Deserialize, ToSchema)]
pub struct AddCommentRequest {
    pub body: Option<String>,
}

impl Validate for AddCommentRequest {
    fn validate(&self, v: &mut Checks) {
        v.text("body", &self.body, 1, 4000);
    }
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct SearchProjectsParams {
    pub stage: Option<String>,
    pub team_id: Option<i32>,
    pub client_id: Option<i64>,
    pub owner_user_id: Option<i32>,
    pub overdue: Option<bool>,
    pub due_from: Option<NaiveDate>,
    pub due_to: Option<NaiveDate>,
    pub cursor: Option<String>,
    /// 1-200, default 50.
    pub size: Option<i32>,
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct CountProjectsParams {
    pub stage: Option<String>,
    pub team_id: Option<i32>,
    pub client_id: Option<i64>,
    pub owner_user_id: Option<i32>,
    pub overdue: Option<bool>,
    pub due_from: Option<NaiveDate>,
    pub due_to: Option<NaiveDate>,
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListProjectCommentsParams {
    pub cursor: Option<String>,
    /// 1-200, default 50.
    pub size: Option<i32>,
}

fn project_row(r: &Row) -> ProjectResponse {
    ProjectResponse {
        id: r.get("id"),
        name: r.get("name"),
        stage: r.get("stage"),
        start_date: r.get("start_date"),
        due_date: r.get("due_date"),
        budget: r.get("budget"),
        overdue: r.get("is_overdue"),
        stage_changed_at: r.get("stage_changed_at"),
        client_id: r.get("client_id"),
        client_name: r.get("client_name"),
        client_city: r.get("client_city"),
        team_id: r.get("team_id"),
        team_name: r.get("team_name"),
        owner_user_id: r.get("owner_user_id"),
        owner_login: r.get("owner_login"),
        owner_display_name: r.get("owner_display_name"),
        updated_at: r.get("updated_at"),
        row_version: r.get("row_version"),
    }
}

fn comment_row(r: &Row) -> CommentResponse {
    CommentResponse {
        id: r.get("id"),
        project_id: r.get("project_id"),
        author_id: r.get("author_id"),
        author_login: r.get("author_login"),
        author_display_name: r.get("author_display_name"),
        body: r.get("body"),
        created_at: r.get("created_at"),
    }
}

fn invalid_cursor() -> ApiError {
    ApiError::bad_request("Invalid cursor.")
}

// ---- repository ------------------------------------------------------------------------------------------

async fn get<C: GenericClient + Sync>(
    client: &C,
    actor_id: i32,
    id: i64,
) -> ApiResult<ProjectResponse> {
    let row = db::query_one(
        client,
        "SELECT * FROM app.project_get($1, $2)",
        &[&actor_id, &id],
    )
    .await?;
    Ok(project_row(&row))
}

// ---- endpoints -------------------------------------------------------------------------------------------

/// Projects page ordered by due date. SALES users see own projects, projects of their teams and projects of
/// their clients.
#[utoipa::path(get, path = "/api/projects", tag = TAG, operation_id = "searchProjects",
    params(SearchProjectsParams), responses((status = 200, body = PageResponseProjectResponse)))]
async fn search_projects(
    State(state): State<AppState>,
    Actor(actor): Actor,
    Params(q): Params<SearchProjectsParams>,
) -> ApiResult<Json<PageResponseProjectResponse>> {
    let size = page_size(q.size)?;
    let (after_due_date, after_id): (Option<NaiveDate>, Option<i64>) =
        match cursor::decode(q.cursor.as_deref(), 2)? {
            Some(p) => (
                Some(p[0].parse().map_err(|_| invalid_cursor())?),
                Some(p[1].parse().map_err(|_| invalid_cursor())?),
            ),
            None => (None, None),
        };
    let overdue = q.overdue.unwrap_or(false);
    let client = state.db.get().await?;
    let rows = db::query(
        &client,
        "SELECT * FROM app.project_search($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        &[
            &actor.id,
            &q.stage,
            &q.team_id,
            &q.client_id,
            &q.owner_user_id,
            &overdue,
            &q.due_from,
            &q.due_to,
            &after_due_date,
            &after_id,
            &size,
        ],
    )
    .await?;
    let items = rows.iter().map(project_row).collect();
    Ok(Json(PageResponseProjectResponse::new(cursor::page(
        items,
        size,
        |p| cursor::encode(&[&p.due_date, &p.id]),
    ))))
}

/// Number of projects matching the same filters as the list. Reads every matching row, so call it only when
/// the user asks for the total.
#[utoipa::path(get, path = "/api/projects/count", tag = TAG, operation_id = "countProjects",
    params(CountProjectsParams), responses((status = 200, body = TotalResponse)))]
async fn count_projects(
    State(state): State<AppState>,
    Actor(actor): Actor,
    Params(q): Params<CountProjectsParams>,
) -> ApiResult<Json<TotalResponse>> {
    let overdue = q.overdue.unwrap_or(false);
    let client = state.db.get().await?;
    let row = db::query_one(
        &client,
        "SELECT total FROM app.project_count($1, $2, $3, $4, $5, $6, $7, $8)",
        &[
            &actor.id,
            &q.stage,
            &q.team_id,
            &q.client_id,
            &q.owner_user_id,
            &overdue,
            &q.due_from,
            &q.due_to,
        ],
    )
    .await?;
    Ok(Json(TotalResponse {
        total: row.get("total"),
    }))
}

/// Project details.
#[utoipa::path(get, path = "/api/projects/{id}", tag = TAG, operation_id = "getProject",
    params(("id" = i64, Path)), responses((status = 200, body = ProjectResponse)))]
async fn get_project(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
) -> ApiResult<Json<ProjectResponse>> {
    let client = state.db.get().await?;
    Ok(Json(get(&client, actor.id, id).await?))
}

/// Create a project (stage NEW). The owner must be a member of the team.
#[utoipa::path(post, path = "/api/projects", tag = TAG, operation_id = "createProject",
    request_body = CreateProjectRequest, responses((status = 201, body = ProjectResponse)))]
async fn create_project(
    State(state): State<AppState>,
    Actor(actor): Actor,
    Body(b): Body<CreateProjectRequest>,
) -> ApiResult<(StatusCode, Json<ProjectResponse>)> {
    let mut client = state.db.get().await?;
    let tx = client.transaction().await?;
    let row = db::query_one(
        &tx,
        "CALL app.project_create($1, $2, $3, $4, $5, $6, $7, $8, NULL)",
        &[
            &actor.id,
            &b.client_id,
            &b.team_id,
            &b.owner_user_id,
            &b.name,
            &b.start_date,
            &b.due_date,
            &b.budget,
        ],
    )
    .await?;
    let project = get(&tx, actor.id, row.get("p_id")).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(project)))
}

/// Update project details (not the stage). `rowVersion` from the last read is required.
#[utoipa::path(put, path = "/api/projects/{id}", tag = TAG, operation_id = "updateProject",
    params(("id" = i64, Path)), request_body = UpdateProjectRequest, responses((status = 200, body = ProjectResponse)))]
async fn update_project(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
    Body(b): Body<UpdateProjectRequest>,
) -> ApiResult<Json<ProjectResponse>> {
    let mut client = state.db.get().await?;
    let tx = client.transaction().await?;
    db::execute(
        &tx,
        "CALL app.project_update($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        &[
            &actor.id,
            &id,
            &b.team_id,
            &b.owner_user_id,
            &b.name,
            &b.start_date,
            &b.due_date,
            &b.budget,
            &b.row_version,
        ],
    )
    .await?;
    let project = get(&tx, actor.id, id).await?;
    tx.commit().await?;
    Ok(Json(project))
}

/// Move the project to another stage, optionally with a comment. Allowed: NEW->ACCEPTED|DECLINED,
/// ACCEPTED->ACTIVE|DECLINED, ACTIVE->ON_HOLD|CLOSED, ON_HOLD->ACTIVE|CLOSED.
#[utoipa::path(post, path = "/api/projects/{id}/stage", tag = TAG, operation_id = "changeProjectStage",
    params(("id" = i64, Path)), request_body = ChangeStageRequest, responses((status = 200, body = ProjectResponse)))]
async fn change_project_stage(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
    Body(b): Body<ChangeStageRequest>,
) -> ApiResult<Json<ProjectResponse>> {
    let mut client = state.db.get().await?;
    let tx = client.transaction().await?;
    db::execute(
        &tx,
        "CALL app.project_change_stage($1, $2, $3, $4, $5)",
        &[&actor.id, &id, &b.stage, &b.comment, &b.row_version],
    )
    .await?;
    let project = get(&tx, actor.id, id).await?;
    tx.commit().await?;
    Ok(Json(project))
}

/// Stage history of the project, newest first.
#[utoipa::path(get, path = "/api/projects/{id}/stages", tag = TAG, operation_id = "listProjectStages",
    params(("id" = i64, Path)), responses((status = 200, body = Vec<StageHistoryEntry>)))]
async fn list_project_stages(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
) -> ApiResult<Json<Vec<StageHistoryEntry>>> {
    let client = state.db.get().await?;
    let rows = db::query(
        &client,
        "SELECT * FROM app.project_stage_history_list($1, $2)",
        &[&actor.id, &id],
    )
    .await?;
    Ok(Json(
        rows.iter()
            .map(|r| StageHistoryEntry {
                id: r.get("id"),
                from_stage: r.get("from_stage"),
                to_stage: r.get("to_stage"),
                changed_at: r.get("changed_at"),
                changed_by: r.get("changed_by"),
                changed_by_login: r.get("changed_by_login"),
            })
            .collect(),
    ))
}

/// Comments of the project, newest first.
#[utoipa::path(get, path = "/api/projects/{id}/comments", tag = TAG, operation_id = "listProjectComments",
    params(("id" = i64, Path), ListProjectCommentsParams),
    responses((status = 200, body = PageResponseCommentResponse)))]
async fn list_project_comments(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
    Params(q): Params<ListProjectCommentsParams>,
) -> ApiResult<Json<PageResponseCommentResponse>> {
    let size = page_size(q.size)?;
    let before_id: Option<i64> = match cursor::decode(q.cursor.as_deref(), 1)? {
        Some(p) => Some(p[0].parse().map_err(|_| invalid_cursor())?),
        None => None,
    };
    let client = state.db.get().await?;
    let rows = db::query(
        &client,
        "SELECT * FROM app.project_comment_list($1, $2, $3, $4)",
        &[&actor.id, &id, &before_id, &size],
    )
    .await?;
    let items = rows.iter().map(comment_row).collect();
    Ok(Json(PageResponseCommentResponse::new(cursor::page(
        items,
        size,
        |c| cursor::encode(&[&c.id]),
    ))))
}

/// Add a comment to the project.
#[utoipa::path(post, path = "/api/projects/{id}/comments", tag = TAG, operation_id = "addProjectComment",
    params(("id" = i64, Path)), request_body = AddCommentRequest, responses((status = 201, body = CommentResponse)))]
async fn add_project_comment(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i64>,
    Body(b): Body<AddCommentRequest>,
) -> ApiResult<(StatusCode, Json<CommentResponse>)> {
    let mut client = state.db.get().await?;
    let tx = client.transaction().await?;
    let row = db::query_one(
        &tx,
        "CALL app.project_comment_add($1, $2, $3, NULL)",
        &[&actor.id, &id, &b.body],
    )
    .await?;
    let comment_id: i64 = row.get("p_id");
    let comment = db::query_one(
        &tx,
        "SELECT * FROM app.project_comment_get($1, $2, $3)",
        &[&actor.id, &id, &comment_id],
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(comment_row(&comment))))
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(search_projects, create_project))
        .routes(routes!(count_projects))
        .routes(routes!(get_project, update_project))
        .routes(routes!(change_project_stage))
        .routes(routes!(list_project_stages))
        .routes(routes!(list_project_comments, add_project_comment))
}

//! Teams and their members. Writes are `CALL app.team_*`; reads `app.team_list`, `app.team_get`,
//! `app.team_member_list` and `app.team_project_stats` (ADR-0015).

use axum::{Json, extract::State, http::StatusCode};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    AppState,
    auth::session::{Actor, Admin},
    db,
    error::{ApiError, ApiResult},
    page_response,
    shared::{Body, Checks, Params, PathParam, Validate, cursor, page_size, ts},
};

pub const TAG: &str = "Teams";

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamResponse {
    pub id: i32,
    pub name: String,
    pub lead_user_id: i32,
    pub lead_login: String,
    pub member_count: i32,
    #[serde(serialize_with = "ts::local")]
    pub created_at: DateTime<Utc>,
}

page_response!(PageResponseTeamResponse, TeamResponse);

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamMemberResponse {
    pub user_id: i32,
    pub login: String,
    pub display_name: String,
    pub role: String,
    pub active: bool,
    #[serde(serialize_with = "ts::local")]
    pub joined_at: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamStageStats {
    pub stage: String,
    pub project_count: i64,
    pub budget_total: Decimal,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTeamRequest {
    pub name: Option<String>,
    pub lead_user_id: Option<i32>,
}

impl Validate for CreateTeamRequest {
    fn validate(&self, v: &mut Checks) {
        v.text("name", &self.name, 1, 200);
        v.present("leadUserId", &self.lead_user_id);
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddMemberRequest {
    pub user_id: Option<i32>,
}

impl Validate for AddMemberRequest {
    fn validate(&self, v: &mut Checks) {
        v.present("userId", &self.user_id);
    }
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListTeamsParams {
    pub cursor: Option<String>,
    /// 1-200, default 50.
    pub size: Option<i32>,
}

fn team_row(r: &Row) -> TeamResponse {
    TeamResponse {
        id: r.get("id"),
        name: r.get("name"),
        lead_user_id: r.get("lead_user_id"),
        lead_login: r.get("lead_login"),
        member_count: r.get("member_count"),
        created_at: r.get("created_at"),
    }
}

/// Teams page, ordered by id.
#[utoipa::path(get, path = "/api/teams", tag = TAG, operation_id = "listTeams", params(ListTeamsParams),
    responses((status = 200, body = PageResponseTeamResponse)))]
async fn list_teams(
    State(state): State<AppState>,
    Actor(actor): Actor,
    Params(q): Params<ListTeamsParams>,
) -> ApiResult<Json<PageResponseTeamResponse>> {
    let size = page_size(q.size)?;
    let after_id: Option<i32> = match cursor::decode(q.cursor.as_deref(), 1)? {
        Some(p) => Some(
            p[0].parse()
                .map_err(|_| ApiError::bad_request("Invalid cursor."))?,
        ),
        None => None,
    };
    let client = state.db.get().await?;
    let rows = db::query(
        &client,
        "SELECT * FROM app.team_list($1, $2, $3)",
        &[&actor.id, &after_id, &size],
    )
    .await?;
    let teams = rows.iter().map(team_row).collect();
    Ok(Json(PageResponseTeamResponse::new(cursor::page(
        teams,
        size,
        |t| cursor::encode(&[&t.id]),
    ))))
}

/// Create a team led by an active manager (ADMIN).
#[utoipa::path(post, path = "/api/teams", tag = TAG, operation_id = "createTeam",
    request_body = CreateTeamRequest, responses((status = 201, body = TeamResponse)))]
async fn create_team(
    State(state): State<AppState>,
    Admin(actor): Admin,
    Body(b): Body<CreateTeamRequest>,
) -> ApiResult<(StatusCode, Json<TeamResponse>)> {
    let mut client = state.db.get().await?;
    let tx = client.transaction().await?;
    let row = db::query_one(
        &tx,
        "CALL app.team_create($1, $2, $3, NULL)",
        &[&actor.id, &b.name, &b.lead_user_id],
    )
    .await?;
    let id: i32 = row.get("p_id");
    let team = db::query_one(&tx, "SELECT * FROM app.team_get($1, $2)", &[&actor.id, &id]).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(team_row(&team))))
}

/// Members of the team.
#[utoipa::path(get, path = "/api/teams/{id}/members", tag = TAG, operation_id = "listTeamMembers",
    params(("id" = i32, Path)), responses((status = 200, body = Vec<TeamMemberResponse>)))]
async fn list_team_members(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i32>,
) -> ApiResult<Json<Vec<TeamMemberResponse>>> {
    let client = state.db.get().await?;
    let rows = db::query(
        &client,
        "SELECT * FROM app.team_member_list($1, $2)",
        &[&actor.id, &id],
    )
    .await?;
    Ok(Json(
        rows.iter()
            .map(|r| TeamMemberResponse {
                user_id: r.get("user_id"),
                login: r.get("login"),
                display_name: r.get("display_name"),
                role: r.get("role"),
                active: r.get("is_active"),
                joined_at: r.get("joined_at"),
            })
            .collect(),
    ))
}

/// Add a member (ADMIN or the team lead).
#[utoipa::path(post, path = "/api/teams/{id}/members", tag = TAG, operation_id = "addTeamMember",
    params(("id" = i32, Path)), request_body = AddMemberRequest, responses((status = 204, description = "Added")))]
async fn add_team_member(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i32>,
    Body(b): Body<AddMemberRequest>,
) -> ApiResult<StatusCode> {
    let client = state.db.get().await?;
    db::execute(
        &client,
        "CALL app.team_member_add($1, $2, $3)",
        &[&actor.id, &id, &b.user_id],
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Remove a member (ADMIN or the team lead); not while they own open projects of the team.
#[utoipa::path(delete, path = "/api/teams/{id}/members/{userId}", tag = TAG, operation_id = "removeTeamMember",
    params(("id" = i32, Path), ("userId" = i32, Path)), responses((status = 204, description = "Removed")))]
async fn remove_team_member(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam((id, user_id)): PathParam<(i32, i32)>,
) -> ApiResult<StatusCode> {
    let client = state.db.get().await?;
    db::execute(
        &client,
        "CALL app.team_member_remove($1, $2, $3)",
        &[&actor.id, &id, &user_id],
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Projects and budget of the team per stage.
#[utoipa::path(get, path = "/api/teams/{id}/stats", tag = TAG, operation_id = "getTeamStats",
    params(("id" = i32, Path)), responses((status = 200, body = Vec<TeamStageStats>)))]
async fn get_team_stats(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(id): PathParam<i32>,
) -> ApiResult<Json<Vec<TeamStageStats>>> {
    let client = state.db.get().await?;
    let rows = db::query(
        &client,
        "SELECT * FROM app.team_project_stats($1, $2)",
        &[&actor.id, &id],
    )
    .await?;
    Ok(Json(
        rows.iter()
            .map(|r| TeamStageStats {
                stage: r.get("stage"),
                project_count: r.get("project_count"),
                budget_total: r.get("budget_total"),
            })
            .collect(),
    ))
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_teams, create_team))
        .routes(routes!(list_team_members, add_team_member))
        .routes(routes!(remove_team_member))
        .routes(routes!(get_team_stats))
}

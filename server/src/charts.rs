//! The numbers behind the desk charts (ADR-0011), as Halcyon counted them: one call of `app.desk_chart_tickets`
//! loads every ticket open at some moment of the range or reported in it, and everything is counted here - fine for
//! a team desk. The deadline states and the Warsaw weekday and hour come from the function (one rule, one clock);
//! this module only buckets and aggregates. The browser draws the charts (no chart library).

use std::collections::HashMap;

use axum::{Json, extract::State};
use chrono::{DateTime, Datelike, Days, NaiveDate, TimeZone, Utc};
use chrono_tz::Europe::Warsaw;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    AppState,
    auth::session::Actor,
    db,
    error::ApiResult,
    reports,
    shared::{Params, ts},
};

pub const TAG: &str = "Desk charts";
const PRIORITIES: [&str; 4] = ["CRITICAL", "HIGH", "MEDIUM", "LOW"];
pub const MIN_FOR_P90: usize = 5;
pub const TOP_AGENTS: usize = 8;

/// The range of the charts page: the last 7 or 30 Warsaw days by day, or the last 12 weeks (Monday to Sunday).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, ToSchema)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
pub enum ChartRange {
    DAYS_7,
    DAYS_30,
    WEEKS_12,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FlowPoint {
    /// First Warsaw day of the bucket (included).
    pub from: NaiveDate,
    /// Last Warsaw day of the bucket (included).
    pub to: NaiveDate,
    pub reported: i32,
    pub resolved: i32,
    pub resolved_late: i32,
    /// Open at the end of the bucket; for the bucket holding now, open now.
    pub open_at_end: i32,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KeyFigures {
    pub reported: i32,
    pub resolved: i32,
    pub resolved_late: i32,
    /// Resolved on time / resolved; null when nothing was resolved.
    pub on_time_share: Option<f64>,
    pub median_resolution_minutes: Option<i64>,
    pub open_now: i32,
    pub due_soon_now: i32,
    pub overdue_now: i32,
}

/// Time to resolve as a share of the ticket's SLA (1 = the whole SLA); p90 only from 5 tickets.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SlaUse {
    pub priority: String,
    pub count: i32,
    pub median: Option<f64>,
    pub p90: Option<f64>,
    pub median_minutes: Option<i64>,
}

/// A stacked bar: one value per segment, in the chart's segment order. `label` only where it is not a catalog key.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Breakdown {
    pub key: String,
    pub label: Option<String>,
    pub values: Vec<i32>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChartData {
    #[schema(inline)]
    pub range: ChartRange,
    pub category_key: Option<String>,
    pub from: NaiveDate,
    pub to: NaiveDate,
    #[serde(serialize_with = "ts::instant")]
    pub generated_at: DateTime<Utc>,
    pub figures: KeyFigures,
    pub flow: Vec<FlowPoint>,
    pub sla: Vec<SlaUse>,
    /// With a category chosen, the breakdown is by its subcategories.
    pub by_subcategory: bool,
    /// Segments: resolved on time, resolved late, open.
    pub by_category: Vec<Breakdown>,
    /// Segments: on time, due soon, overdue - open tickets now, per priority.
    pub queue: Vec<Breakdown>,
    /// Segments: resolved on time, resolved late - top 8 agents and the rest.
    pub agents: Vec<Breakdown>,
    /// Reports per Warsaw weekday (0 = Monday) and hour: 7 rows of 24.
    pub heat: Vec<Vec<i32>>,
}

/// One row of `app.desk_chart_tickets`.
pub struct ChartTicket {
    pub category_key: String,
    pub subcategory_key: String,
    pub priority: String,
    pub status: String,
    pub reported_at: DateTime<Utc>,
    pub deadline: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_late: bool,
    pub assignee_user_id: Option<i32>,
    pub assignee_name: Option<String>,
    pub overdue: bool,
    pub due_soon: bool,
    pub weekday: i32,
    pub hour: i32,
}

/// An agent of the agents' chart: user id and name, as the ticket recorded them.
type AgentKey = (Option<i32>, String);

pub fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let mid = sorted.len() / 2;
    Some(if sorted.len() % 2 == 1 {
        sorted[mid]
    } else {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    })
}

/// The nearest rank: the smallest value with at least `p` of the values at or below it.
pub fn percentile(values: &[f64], p: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = ((p * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
    Some(sorted[rank - 1])
}

fn start(day: NaiveDate) -> DateTime<Utc> {
    let midnight = day.and_hms_opt(0, 0, 0).unwrap_or_default();
    Warsaw
        .from_local_datetime(&midnight)
        .earliest()
        .map(|t| t.with_timezone(&Utc))
        .unwrap_or_else(|| Utc.from_utc_datetime(&midnight))
}

fn minutes(from: DateTime<Utc>, to: DateTime<Utc>) -> f64 {
    (to - from).num_seconds() as f64 / 60.0
}

/// [first day, the day after the last) of each bucket.
pub fn buckets(range: ChartRange, today: NaiveDate) -> Vec<(NaiveDate, NaiveDate)> {
    let days = |n: u64| -> Vec<(NaiveDate, NaiveDate)> {
        (0..n)
            .rev()
            .map(|i| today - Days::new(i))
            .map(|d| (d, d + Days::new(1)))
            .collect()
    };
    match range {
        ChartRange::DAYS_7 => days(7),
        ChartRange::DAYS_30 => days(30),
        ChartRange::WEEKS_12 => {
            let monday = today - Days::new(u64::from(today.weekday().num_days_from_monday()));
            (0..12)
                .rev()
                .map(|i| monday - Days::new(7 * i))
                .map(|d| (d, d + Days::new(7)))
                .collect()
        }
    }
}

fn count<T>(items: &[&T], predicate: impl Fn(&T) -> bool) -> i32 {
    items.iter().filter(|t| predicate(t)).count() as i32
}

pub fn build(
    range: ChartRange,
    category_key: Option<String>,
    buckets: &[(NaiveDate, NaiveDate)],
    tickets: &[ChartTicket],
    now: DateTime<Utc>,
) -> ChartData {
    let first = buckets[0].0;
    let last = buckets[buckets.len() - 1].1.pred_opt().unwrap_or(first);
    let from = start(first);
    let until = start(buckets[buckets.len() - 1].1);
    let in_range = |at: DateTime<Utc>| at >= from && at < until;

    let reported: Vec<&ChartTicket> = tickets.iter().filter(|t| in_range(t.reported_at)).collect();
    let resolved: Vec<&ChartTicket> = tickets
        .iter()
        .filter(|t| t.resolved_at.is_some_and(in_range))
        .collect();
    let open: Vec<&ChartTicket> = tickets.iter().filter(|t| t.status != "DONE").collect();
    let resolved_at = |t: &ChartTicket| t.resolved_at.unwrap_or(t.reported_at);

    let flow = buckets
        .iter()
        .map(|(day, next)| {
            let (b0, b1) = (start(*day), start(*next));
            let open_at_end = if b1 <= now {
                tickets
                    .iter()
                    .filter(|t| t.reported_at < b1 && t.resolved_at.is_none_or(|r| r >= b1))
                    .count() as i32
            } else {
                // The bucket holding now: open now, so the last point equals the key figure.
                tickets
                    .iter()
                    .filter(|t| t.reported_at <= now && t.status != "DONE")
                    .count() as i32
            };
            let within = |at: DateTime<Utc>| at >= b0 && at < b1;
            FlowPoint {
                from: *day,
                to: next.pred_opt().unwrap_or(*day),
                reported: count(&reported, |t| within(t.reported_at)),
                resolved: count(&resolved, |t| within(resolved_at(t))),
                resolved_late: count(&resolved, |t| t.resolved_late && within(resolved_at(t))),
                open_at_end,
            }
        })
        .collect();

    let durations: Vec<f64> = resolved
        .iter()
        .map(|t| minutes(t.reported_at, resolved_at(t)))
        .collect();
    let figures = KeyFigures {
        reported: reported.len() as i32,
        resolved: resolved.len() as i32,
        resolved_late: count(&resolved, |t| t.resolved_late),
        on_time_share: if resolved.is_empty() {
            None
        } else {
            Some(count(&resolved, |t| !t.resolved_late) as f64 / resolved.len() as f64)
        },
        median_resolution_minutes: median(&durations).map(|m| m as i64),
        open_now: open.len() as i32,
        due_soon_now: count(&open, |t| t.due_soon),
        overdue_now: count(&open, |t| t.overdue),
    };

    let sla = PRIORITIES
        .iter()
        .map(|p| {
            let of: Vec<&ChartTicket> = resolved
                .iter()
                .copied()
                .filter(|t| t.priority == *p)
                .collect();
            let shares: Vec<f64> = of
                .iter()
                .map(|t| {
                    minutes(t.reported_at, resolved_at(t)) / minutes(t.reported_at, t.deadline)
                })
                .collect();
            let took: Vec<f64> = of
                .iter()
                .map(|t| minutes(t.reported_at, resolved_at(t)))
                .collect();
            SlaUse {
                priority: (*p).to_owned(),
                count: of.len() as i32,
                median: median(&shares),
                p90: if of.len() >= MIN_FOR_P90 {
                    percentile(&shares, 0.9)
                } else {
                    None
                },
                median_minutes: median(&took).map(|m| m as i64),
            }
        })
        .collect();

    let by_subcategory = category_key.is_some();
    let mut groups: HashMap<&str, Vec<&ChartTicket>> = HashMap::new();
    for t in &reported {
        let key = if by_subcategory {
            &t.subcategory_key
        } else {
            &t.category_key
        };
        groups.entry(key.as_str()).or_default().push(t);
    }
    let mut by_category: Vec<Breakdown> = groups
        .into_iter()
        .map(|(key, group)| Breakdown {
            key: key.to_owned(),
            label: None,
            values: vec![
                count(&group, |t| t.status == "DONE" && !t.resolved_late),
                count(&group, |t| t.status == "DONE" && t.resolved_late),
                count(&group, |t| t.status != "DONE"),
            ],
        })
        .collect();
    by_category.sort_by(|a, b| {
        b.values
            .iter()
            .sum::<i32>()
            .cmp(&a.values.iter().sum::<i32>())
            .then_with(|| a.key.cmp(&b.key))
    });

    let queue = PRIORITIES
        .iter()
        .map(|p| {
            let of: Vec<&ChartTicket> = open.iter().copied().filter(|t| t.priority == *p).collect();
            Breakdown {
                key: (*p).to_owned(),
                label: None,
                values: vec![
                    count(&of, |t| !t.overdue && !t.due_soon),
                    count(&of, |t| t.due_soon),
                    count(&of, |t| t.overdue),
                ],
            }
        })
        .collect();

    let mut per_agent: Vec<(AgentKey, Vec<&ChartTicket>)> = Vec::new();
    for t in &resolved {
        let agent = (
            t.assignee_user_id,
            t.assignee_name.clone().unwrap_or_else(|| "?".into()),
        );
        match per_agent.iter_mut().find(|(a, _)| *a == agent) {
            Some((_, group)) => group.push(t),
            None => per_agent.push((agent, vec![t])),
        }
    }
    let mut agents: Vec<Breakdown> = per_agent
        .into_iter()
        .map(|((id, name), group)| Breakdown {
            key: id.map(|i| i.to_string()).unwrap_or_default(),
            label: Some(name),
            values: vec![
                count(&group, |t| !t.resolved_late),
                count(&group, |t| t.resolved_late),
            ],
        })
        .collect();
    agents.sort_by(|a, b| {
        b.values
            .iter()
            .sum::<i32>()
            .cmp(&a.values.iter().sum::<i32>())
            .then_with(|| a.label.cmp(&b.label))
    });
    if agents.len() > TOP_AGENTS + 1 {
        let rest = agents.split_off(TOP_AGENTS);
        agents.push(Breakdown {
            key: "rest".into(),
            label: Some(rest.len().to_string()),
            values: vec![
                rest.iter().map(|b| b.values[0]).sum(),
                rest.iter().map(|b| b.values[1]).sum(),
            ],
        });
    }

    let mut heat = vec![vec![0; 24]; 7];
    for t in &reported {
        if let Some(cell) = heat
            .get_mut(t.weekday as usize)
            .and_then(|row| row.get_mut(t.hour as usize))
        {
            *cell += 1;
        }
    }

    ChartData {
        range,
        category_key,
        from: first,
        to: last,
        generated_at: now,
        figures,
        flow,
        sla,
        by_subcategory,
        by_category,
        queue,
        agents,
        heat,
    }
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct DeskChartsParams {
    /// DAYS_7, DAYS_30 (default) or WEEKS_12.
    #[param(inline)]
    pub range: Option<ChartRange>,
    pub category_key: Option<String>,
}

/// The numbers of the charts page: key figures, flow, SLA use, by category, queue now, agents and the weekday x
/// hour heatmap, for the last 7 / 30 days or 12 weeks, optionally of one category.
#[utoipa::path(get, path = "/api/desk/charts", tag = TAG, operation_id = "getDeskCharts",
    params(DeskChartsParams), responses((status = 200, body = ChartData)))]
async fn get_desk_charts(
    State(state): State<AppState>,
    Actor(actor): Actor,
    Params(q): Params<DeskChartsParams>,
) -> ApiResult<Json<ChartData>> {
    let range = q.range.unwrap_or(ChartRange::DAYS_30);
    let buckets = buckets(range, reports::today());
    let first = buckets[0].0;
    let last = buckets[buckets.len() - 1].1.pred_opt().unwrap_or(first);
    let client = state.db.get().await?;
    let rows = db::query(
        &client,
        "SELECT category_key, subcategory_key, priority, status, reported_at, deadline, resolved_at, resolved_late, \
         assignee_user_id, assignee_name, is_overdue, is_due_soon, report_weekday, report_hour, now_utc \
         FROM app.desk_chart_tickets($1, $2, $3, $4)",
        &[&actor.id, &first, &last, &q.category_key],
    )
    .await?;
    let now = rows
        .first()
        .map(|r| r.get::<_, DateTime<Utc>>("now_utc"))
        .unwrap_or_else(Utc::now);
    let tickets: Vec<ChartTicket> = rows
        .iter()
        .map(|r| ChartTicket {
            category_key: r.get("category_key"),
            subcategory_key: r.get("subcategory_key"),
            priority: r.get("priority"),
            status: r.get("status"),
            reported_at: r.get("reported_at"),
            deadline: r.get("deadline"),
            resolved_at: r.get("resolved_at"),
            resolved_late: r.get("resolved_late"),
            assignee_user_id: r.get("assignee_user_id"),
            assignee_name: r.get("assignee_name"),
            overdue: r.get("is_overdue"),
            due_soon: r.get("is_due_soon"),
            weekday: r.get("report_weekday"),
            hour: r.get("report_hour"),
        })
        .collect();
    Ok(Json(build(range, q.category_key, &buckets, &tickets, now)))
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(get_desk_charts))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_and_nearest_rank_p90() {
        assert_eq!(median(&[]), None);
        assert_eq!(median(&[3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(&[4.0, 1.0, 2.0, 3.0]), Some(2.5));
        let ten: Vec<f64> = (1..=10).map(f64::from).collect();
        assert_eq!(percentile(&ten, 0.9), Some(9.0));
        assert_eq!(percentile(&[5.0], 0.9), Some(5.0));
    }

    #[test]
    fn buckets_of_each_range() {
        let wednesday = NaiveDate::from_ymd_opt(2026, 11, 11).unwrap();
        let days = buckets(ChartRange::DAYS_7, wednesday);
        assert_eq!(days.len(), 7);
        assert_eq!(
            days[6],
            (wednesday, NaiveDate::from_ymd_opt(2026, 11, 12).unwrap())
        );
        let weeks = buckets(ChartRange::WEEKS_12, wednesday);
        assert_eq!(weeks.len(), 12);
        assert_eq!(
            weeks[11].0,
            NaiveDate::from_ymd_opt(2026, 11, 9).unwrap(),
            "this week's Monday"
        );
        assert_eq!(buckets(ChartRange::DAYS_30, wednesday).len(), 30);
    }

    #[test]
    fn warsaw_midnight_moves_with_daylight_saving() {
        let summer = start(NaiveDate::from_ymd_opt(2026, 7, 1).unwrap());
        let winter = start(NaiveDate::from_ymd_opt(2026, 12, 1).unwrap());
        assert_eq!(summer.to_rfc3339(), "2026-06-30T22:00:00+00:00");
        assert_eq!(winter.to_rfc3339(), "2026-11-30T23:00:00+00:00");
    }
}

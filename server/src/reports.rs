//! Desk reports (ADR-0010) for every agent: five category reports with their report data and the agents' work. The
//! page and the exports get the same [Table]; `lang` (pl / en) names its headers and catalog values. The data and its
//! rules come from the `app.desk_report_*` functions; this module names the columns, pivots the report fields into
//! columns (only the fields that have a value in at least one row, in catalog order) and translates catalog values
//! for English.

use std::{collections::HashMap, str::FromStr, sync::LazyLock};

use axum::extract::State;
use axum::{
    Json,
    http::header,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use chrono_tz::Europe::Warsaw;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    AppState,
    auth::session::Actor,
    db,
    desk::{self, FieldDefinitionResponse},
    error::{ApiError, ApiResult},
    exports::{self, ExportFormat},
    shared::{Params, PathParam},
};

pub const TAG: &str = "Desk reports";

/// Polish (the default) or English: the language of a report's headers, catalog names and exports (ADR-0008).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Pl,
    En,
}

impl Lang {
    pub fn of(value: Option<&str>) -> Lang {
        match value {
            Some(v) if v.eq_ignore_ascii_case("en") => Lang::En,
            _ => Lang::Pl,
        }
    }

    pub fn t<'a>(self, pl: &'a str, en: &'a str) -> &'a str {
        if self == Lang::Pl { pl } else { en }
    }
}

/// How a cell is typed: exports write real dates and numbers, the page formats them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CellKind {
    Text,
    Date,
    DateTime,
    Number,
    Amount,
}

impl CellKind {
    fn name(self) -> &'static str {
        match self {
            CellKind::Text => "TEXT",
            CellKind::Date => "DATE",
            CellKind::DateTime => "DATETIME",
            CellKind::Number => "NUMBER",
            CellKind::Amount => "AMOUNT",
        }
    }
}

/// A typed cell; an instant is shown in Warsaw time.
#[derive(Clone, PartialEq, Debug)]
pub enum Cell {
    Text(String),
    Date(NaiveDate),
    Instant(DateTime<Utc>),
    Number(i64),
    Amount(Decimal),
}

impl Cell {
    /// The cell for the page: ISO dates and instants, plain numbers.
    pub fn text(&self) -> String {
        match self {
            Cell::Text(s) => s.clone(),
            Cell::Date(d) => d.to_string(),
            Cell::Instant(at) => at.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
            Cell::Number(n) => n.to_string(),
            Cell::Amount(a) => a.to_string(),
        }
    }
}

fn text(value: Option<String>) -> Option<Cell> {
    value.map(Cell::Text)
}

/// A column; `group` is the header above it (a period of the agents' report).
#[derive(Clone, Debug)]
pub struct Column {
    pub key: String,
    pub label: String,
    pub kind: CellKind,
    pub group: Option<String>,
}

impl Column {
    fn new(key: impl Into<String>, label: &str, kind: CellKind) -> Self {
        Column {
            key: key.into(),
            label: label.to_owned(),
            kind,
            group: None,
        }
    }

    /// The header of exports: "group: label".
    pub fn header(&self) -> String {
        match &self.group {
            Some(g) => format!("{g}: {}", self.label),
            None => self.label.clone(),
        }
    }
}

/// The one table a report page shows and every export writes (ADR-0010).
#[derive(Clone, Debug)]
pub struct Table {
    pub slug: String,
    pub title: String,
    pub subtitle: String,
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<Option<Cell>>>,
    /// A last row of totals (the agents' report).
    pub totals: Option<Vec<Option<Cell>>>,
}

/// English names of the desk catalog (ADR-0008), keyed by the Polish name stored in the database; generated from
/// Halcyon's catalog, the same data as the web app's catalogEn.ts. A name missing here stays Polish.
static ENGLISH: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../assets/desk/catalog-en.json"))
        .expect("assets/desk/catalog-en.json is a map of names")
});

fn name(value: &str, lang: Lang) -> String {
    match lang {
        Lang::En => ENGLISH
            .get(value)
            .cloned()
            .unwrap_or_else(|| value.to_owned()),
        Lang::Pl => value.to_owned(),
    }
}

/// A category report: its slug in URLs (as in Halcyon) and the category it reads.
const CATEGORY_REPORTS: &[(&str, &str)] = &[
    ("kadry-polska", "hr-poland"),
    ("kadry-zagranica", "hr-abroad"),
    ("finanse", "finance"),
    ("rekrutacja", "recruitment"),
    ("marketing", "marketing"),
];

fn unknown_report(what: &str) -> ApiError {
    ApiError::not_found(format!("Unknown report: {what}"))
}

pub fn status_label(status: &str, late: bool, lang: Lang) -> &'static str {
    match status {
        "IN_POOL" => lang.t("W puli", "In the pool"),
        "IN_PROGRESS" => lang.t("W realizacji", "In progress"),
        _ if late => lang.t("Zrealizowane po terminie", "Resolved late"),
        _ => lang.t("Zrealizowane", "Resolved"),
    }
}

pub fn priority_label(priority: &str, lang: Lang) -> &'static str {
    match priority {
        "CRITICAL" => lang.t("Krytyczny", "Critical"),
        "HIGH" => lang.t("Wysoki", "High"),
        "MEDIUM" => lang.t("Średni", "Medium"),
        _ => lang.t("Niski", "Low"),
    }
}

/// Short month names as Java's pl-PL and en-GB formats print them.
const MONTHS_PL: [&str; 12] = [
    "sty", "lut", "mar", "kwi", "maj", "cze", "lip", "sie", "wrz", "paź", "lis", "gru",
];
const MONTHS_EN: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sept", "Oct", "Nov", "Dec",
];

/// "2 lis – 8 lis 2026" / "2 Nov – 8 Nov 2026", both days included; one day is "2 lis 2026".
pub fn period(from: NaiveDate, to: NaiveDate, lang: Lang) -> String {
    let months = if lang == Lang::Pl {
        &MONTHS_PL
    } else {
        &MONTHS_EN
    };
    let day_month = |d: NaiveDate| format!("{} {}", d.day(), months[d.month0() as usize]);
    let full = |d: NaiveDate| format!("{} {}", day_month(d), d.year());
    if from == to {
        full(to)
    } else {
        format!("{} – {}", day_month(from), full(to))
    }
}

/// Today in Warsaw: the day of the export's file name and of the charts' range.
pub fn today() -> NaiveDate {
    Utc::now().with_timezone(&Warsaw).date_naive()
}

// ---- building the tables -------------------------------------------------------------------------------------

struct CategoryFilter {
    from: NaiveDate,
    to: NaiveDate,
    market: Option<String>,
    subcategory_key: Option<String>,
    completion: Option<String>,
}

fn kind_of(field: &FieldDefinitionResponse) -> CellKind {
    match field.kind.as_str() {
        "DATE" => CellKind::Date,
        "AMOUNT" => CellKind::Amount,
        "COUNT" => CellKind::Number,
        _ => CellKind::Text,
    }
}

/// A stored value as its typed cell; choices are translated for English, what people typed is not.
fn field_cell(field: &FieldDefinitionResponse, value: &str, lang: Lang) -> Cell {
    let typed = match field.kind.as_str() {
        "DATE" => NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .ok()
            .map(Cell::Date),
        "AMOUNT" => Decimal::from_str(value).ok().map(Cell::Amount),
        "COUNT" => value.parse().ok().map(Cell::Number),
        "CHOICE" => Some(Cell::Text(name(value, lang))),
        "MULTI_CHOICE" => Some(Cell::Text(
            value
                .split(", ")
                .map(|v| name(v, lang))
                .collect::<Vec<_>>()
                .join(", "),
        )),
        _ => None,
    };
    typed.unwrap_or_else(|| Cell::Text(value.to_owned()))
}

async fn category_table(
    state: &AppState,
    actor_id: i32,
    slug: &str,
    filter: CategoryFilter,
    lang: Lang,
) -> ApiResult<Table> {
    let (slug, category_key) = CATEGORY_REPORTS
        .iter()
        .find(|(s, _)| *s == slug)
        .ok_or_else(|| unknown_report(slug))?;
    let catalog = desk::catalog(state).await?;
    let category = catalog
        .categories
        .iter()
        .find(|c| c.key == *category_key)
        .ok_or_else(|| unknown_report(slug))?;
    let client = state.db.get().await?;
    let rows = db::query(
        &client,
        "SELECT number, reported_at, market, subcategory_key, title, requester_incognito, requester_name, status, \
         priority, assignee_name, handler_name, deadline, resolved_at, resolved_late, fields \
         FROM app.desk_report_category($1, $2, $3, $4, $5, $6, $7)",
        &[
            &actor_id,
            category_key,
            &filter.from,
            &filter.to,
            &filter.market,
            &filter.subcategory_key,
            &filter.completion,
        ],
    )
    .await?;

    let mut definitions: Vec<&FieldDefinitionResponse> = Vec::new();
    for field in category.subcategories.iter().flat_map(|s| &s.fields) {
        if !definitions.iter().any(|d| d.key == field.key) {
            definitions.push(field);
        }
    }
    let values: Vec<serde_json::Map<String, serde_json::Value>> = rows
        .iter()
        .map(|r| match r.get::<_, Option<serde_json::Value>>("fields") {
            Some(serde_json::Value::Object(map)) => map,
            _ => serde_json::Map::new(),
        })
        .collect();
    let used: Vec<&FieldDefinitionResponse> = definitions
        .into_iter()
        .filter(|d| values.iter().any(|v| v.contains_key(&d.key)))
        .collect();
    let has_markets = category.subcategories.iter().any(|s| s.market.is_some());

    let mut columns = vec![
        Column::new("number", lang.t("Numer", "Number"), CellKind::Text),
        Column::new(
            "reported",
            lang.t("Data zgłoszenia", "Reported"),
            CellKind::DateTime,
        ),
    ];
    if has_markets {
        columns.push(Column::new(
            "market",
            lang.t("Rynek", "Market"),
            CellKind::Text,
        ));
    }
    for (key, pl, en, kind) in [
        ("subcategory", "Podkategoria", "Subcategory", CellKind::Text),
        ("title", "Tytuł", "Title", CellKind::Text),
        ("requester", "Zgłaszający", "Requester", CellKind::Text),
        ("status", "Status", "Status", CellKind::Text),
        ("priority", "Priorytet", "Priority", CellKind::Text),
        ("agent", "Agent", "Agent", CellKind::Text),
        (
            "handler",
            "Osoba realizująca",
            "Person doing the work",
            CellKind::Text,
        ),
        ("deadline", "Termin", "Deadline", CellKind::DateTime),
        ("resolved", "Zrealizowano", "Resolved", CellKind::DateTime),
    ] {
        columns.push(Column::new(key, lang.t(pl, en), kind));
    }
    for field in &used {
        columns.push(Column::new(
            format!("field:{}", field.key),
            &name(&field.label, lang),
            kind_of(field),
        ));
    }

    let table = rows
        .iter()
        .zip(&values)
        .map(|(r, fields)| {
            let mut cells = vec![
                Some(Cell::Text(r.get("number"))),
                Some(Cell::Instant(r.get("reported_at"))),
            ];
            if has_markets {
                cells.push(
                    r.get::<_, Option<String>>("market")
                        .map(|m| Cell::Text(name(&m, lang))),
                );
            }
            let subcategory_key: String = r.get("subcategory_key");
            let subcategory = category
                .subcategories
                .iter()
                .find(|s| s.key == subcategory_key)
                .map(|s| name(&s.name, lang))
                .unwrap_or(subcategory_key);
            cells.push(Some(Cell::Text(subcategory)));
            cells.push(Some(Cell::Text(r.get("title"))));
            cells.push(if r.get("requester_incognito") {
                Some(Cell::Text("incognito".into()))
            } else {
                text(r.get("requester_name"))
            });
            cells.push(Some(Cell::Text(
                status_label(r.get("status"), r.get("resolved_late"), lang).into(),
            )));
            cells.push(Some(Cell::Text(
                priority_label(r.get("priority"), lang).into(),
            )));
            cells.push(text(r.get("assignee_name")));
            cells.push(text(r.get("handler_name")));
            cells.push(Some(Cell::Instant(r.get("deadline"))));
            cells.push(
                r.get::<_, Option<DateTime<Utc>>>("resolved_at")
                    .map(Cell::Instant),
            );
            for field in &used {
                cells.push(
                    fields
                        .get(&field.key)
                        .and_then(|v| v.as_str())
                        .map(|v| field_cell(field, v, lang)),
                );
            }
            cells
        })
        .collect();

    Ok(Table {
        slug: (*slug).to_owned(),
        title: name(&category.name, lang),
        subtitle: period(filter.from, filter.to, lang),
        columns,
        rows: table,
        totals: None,
    })
}

const AGENT_COUNTS: [&str; 9] = [
    "taken_week",
    "resolved_week",
    "late_week",
    "taken_month",
    "resolved_month",
    "late_month",
    "taken_three",
    "resolved_three",
    "late_three",
];

async fn agents_table(
    state: &AppState,
    actor_id: i32,
    previous: bool,
    lang: Lang,
) -> ApiResult<Table> {
    let client = state.db.get().await?;
    let periods = db::query_one(
        &client,
        "SELECT week_from, week_to, month_from, month_to, three_from, three_to \
         FROM app.desk_report_period_list($1, $2)",
        &[&actor_id, &previous],
    )
    .await?;
    let rows = db::query(
        &client,
        "SELECT display_name, taken_week, resolved_week, late_week, taken_month, resolved_month, late_month, \
         taken_three, resolved_three, late_three, in_progress FROM app.desk_report_agents($1, $2)",
        &[&actor_id, &previous],
    )
    .await?;

    let groups = [
        (lang.t("Tydzień", "Week"), "week"),
        (lang.t("Miesiąc", "Month"), "month"),
        (lang.t("3 miesiące", "3 months"), "three"),
    ];
    let mut columns = vec![Column::new(
        "agent",
        lang.t("Agent", "Agent"),
        CellKind::Text,
    )];
    for (i, (label, prefix)) in groups.iter().enumerate() {
        let from: NaiveDate = periods.get(format!("{prefix}_from").as_str());
        let until: NaiveDate = periods.get(format!("{prefix}_to").as_str());
        let group = format!(
            "{label} {}",
            period(from, until.pred_opt().unwrap_or(until), lang)
        );
        for (key, pl, en) in [
            ("taken", "Przejęte", "Taken"),
            ("resolved", "Zrealizowane", "Resolved"),
            ("late", "Po terminie", "Late"),
        ] {
            let mut column = Column::new(format!("{key}:{i}"), lang.t(pl, en), CellKind::Number);
            column.group = Some(group.clone());
            columns.push(column);
        }
    }
    columns.push(Column::new(
        "inProgress",
        lang.t("W realizacji teraz", "In progress now"),
        CellKind::Number,
    ));

    let counts: Vec<(String, Vec<i64>)> = rows
        .iter()
        .map(|r| {
            let mut n: Vec<i64> = AGENT_COUNTS
                .iter()
                .map(|c| i64::from(r.get::<_, i32>(*c)))
                .collect();
            n.push(i64::from(r.get::<_, i32>("in_progress")));
            (r.get("display_name"), n)
        })
        .collect();
    let table = counts
        .iter()
        .map(|(agent, n)| {
            std::iter::once(Some(Cell::Text(agent.clone())))
                .chain(n.iter().map(|v| Some(Cell::Number(*v))))
                .collect()
        })
        .collect();
    let totals = std::iter::once(Some(Cell::Text(lang.t("Razem", "Total").into())))
        .chain((0..10).map(|c| Some(Cell::Number(counts.iter().map(|(_, n)| n[c]).sum()))))
        .collect();
    Ok(Table {
        slug: "agenci".into(),
        title: lang.t("Praca agentów", "Agents' work").into(),
        subtitle: if previous {
            lang.t("Poprzednie okresy", "Previous periods")
        } else {
            lang.t("Bieżące okresy", "Current periods")
        }
        .into(),
        columns,
        rows: table,
        totals: Some(totals),
    })
}

// ---- endpoints -----------------------------------------------------------------------------------------------

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReportColumnResponse {
    pub key: String,
    pub label: String,
    /// TEXT, DATE, DATETIME, NUMBER or AMOUNT.
    pub kind: String,
    pub group: Option<String>,
}

/// A report for the page: cells as text (ISO dates and instants, plain numbers); `kind` says how to show them.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReportTableResponse {
    pub slug: String,
    pub title: String,
    pub subtitle: String,
    pub columns: Vec<ReportColumnResponse>,
    pub rows: Vec<Vec<Option<String>>>,
    pub totals: Option<Vec<Option<String>>>,
}

impl From<Table> for ReportTableResponse {
    fn from(table: Table) -> Self {
        let cells =
            |row: Vec<Option<Cell>>| row.iter().map(|c| c.as_ref().map(Cell::text)).collect();
        ReportTableResponse {
            slug: table.slug,
            title: table.title,
            subtitle: table.subtitle,
            columns: table
                .columns
                .into_iter()
                .map(|c| ReportColumnResponse {
                    key: c.key,
                    label: c.label,
                    kind: c.kind.name().into(),
                    group: c.group,
                })
                .collect(),
            rows: table.rows.into_iter().map(cells).collect(),
            totals: table.totals.map(cells),
        }
    }
}

/// The body of an export: a file (the name the web client's generated types use).
#[derive(Serialize, ToSchema)]
pub struct StreamingResponseBody {}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct CategoryReportParams {
    /// First Warsaw day of the period (included).
    pub from: NaiveDate,
    /// Last Warsaw day of the period (included); at most one year after `from`.
    pub to: NaiveDate,
    pub market: Option<String>,
    pub subcategory_key: Option<String>,
    /// OPEN or DONE.
    pub completion: Option<String>,
    /// pl (default) or en.
    pub lang: Option<String>,
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AgentReportParams {
    /// The periods just before the current ones.
    pub previous: Option<bool>,
    /// pl (default) or en.
    pub lang: Option<String>,
}

impl CategoryReportParams {
    fn split(self) -> (CategoryFilter, Lang) {
        let lang = Lang::of(self.lang.as_deref());
        (
            CategoryFilter {
                from: self.from,
                to: self.to,
                market: self.market,
                subcategory_key: self.subcategory_key,
                completion: self.completion,
            },
            lang,
        )
    }
}

fn file(table: Table, extension: &str, lang: Lang) -> ApiResult<Response> {
    let format = ExportFormat::of(extension)
        .ok_or_else(|| unknown_report(&format!("export.{extension}")))?;
    let file_name = format!("raport-{}-{}.{}", table.slug, today(), format.extension());
    let bytes = exports::write(&table, format, lang).map_err(ApiError::Internal)?;
    Ok((
        [
            (header::CONTENT_TYPE, format.content_type().to_owned()),
            (
                header::CONTENT_DISPOSITION,
                desk::content_disposition("attachment", &file_name),
            ),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_owned()),
        ],
        bytes,
    )
        .into_response())
}

/// A category report (kadry-polska, kadry-zagranica, finanse, rekrutacja, marketing): tickets reported from..to
/// (Warsaw days, both included) with their report data as columns.
#[utoipa::path(get, path = "/api/desk/reports/categories/{slug}", tag = TAG, operation_id = "getCategoryReport",
    params(("slug" = String, Path), CategoryReportParams), responses((status = 200, body = ReportTableResponse)))]
async fn get_category_report(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(slug): PathParam<String>,
    Params(q): Params<CategoryReportParams>,
) -> ApiResult<Json<ReportTableResponse>> {
    let (filter, lang) = q.split();
    let table = category_table(&state, actor.id, &slug, filter, lang).await?;
    Ok(Json(table.into()))
}

/// The category report as a file: csv, xlsx or pdf.
#[utoipa::path(get, path = "/api/desk/reports/categories/{slug}/export.{format}", tag = TAG,
    operation_id = "exportCategoryReport",
    params(("slug" = String, Path), ("format" = String, Path), CategoryReportParams),
    responses((status = 200, body = StreamingResponseBody)))]
async fn export_category_report(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam((slug, format)): PathParam<(String, String)>,
    Params(q): Params<CategoryReportParams>,
) -> ApiResult<Response> {
    let (filter, lang) = q.split();
    let table = category_table(&state, actor.id, &slug, filter, lang).await?;
    file(table, &format, lang)
}

/// The agents' work: taken, resolved and late per agent in this week, month and 3 months.
#[utoipa::path(get, path = "/api/desk/reports/agents", tag = TAG, operation_id = "getAgentReport",
    params(AgentReportParams), responses((status = 200, body = ReportTableResponse)))]
async fn get_agent_report(
    State(state): State<AppState>,
    Actor(actor): Actor,
    Params(q): Params<AgentReportParams>,
) -> ApiResult<Json<ReportTableResponse>> {
    let lang = Lang::of(q.lang.as_deref());
    let table = agents_table(&state, actor.id, q.previous.unwrap_or(false), lang).await?;
    Ok(Json(table.into()))
}

/// The agents' work as a file: csv, xlsx or pdf.
#[utoipa::path(get, path = "/api/desk/reports/agents/export.{format}", tag = TAG,
    operation_id = "exportAgentReport",
    params(("format" = String, Path), AgentReportParams),
    responses((status = 200, body = StreamingResponseBody)))]
async fn export_agent_report(
    State(state): State<AppState>,
    Actor(actor): Actor,
    PathParam(format): PathParam<String>,
    Params(q): Params<AgentReportParams>,
) -> ApiResult<Response> {
    let lang = Lang::of(q.lang.as_deref());
    let table = agents_table(&state, actor.id, q.previous.unwrap_or(false), lang).await?;
    file(table, &format, lang)
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_category_report))
        .routes(routes!(export_category_report))
        .routes(routes!(get_agent_report))
        .routes(routes!(export_agent_report))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn periods_as_java_printed_them() {
        let d = |y, m, d| NaiveDate::from_ymd_opt(y, m, d).unwrap();
        assert_eq!(
            period(d(2026, 11, 2), d(2026, 11, 8), Lang::Pl),
            "2 lis – 8 lis 2026"
        );
        assert_eq!(
            period(d(2026, 9, 1), d(2026, 9, 30), Lang::En),
            "1 Sept – 30 Sept 2026"
        );
        assert_eq!(period(d(2026, 5, 4), d(2026, 5, 4), Lang::Pl), "4 maj 2026");
    }

    #[test]
    fn english_names_and_a_missing_name_stays_polish() {
        assert_eq!(name("Awaria", Lang::En), "Outage");
        assert_eq!(name("Awaria", Lang::Pl), "Awaria");
        assert_eq!(name("Nie ma takiej", Lang::En), "Nie ma takiej");
    }
}

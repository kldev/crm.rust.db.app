//! The desk reports and their exports (ADR-0010): only fields with a value become columns, English headers and
//! catalog names, the exports carry the same table (CSV parses, XLSX has the same rows, the PDF has Polish letters).

mod support;

use std::io::Cursor;

use calamine::{Data, Reader, Xlsx};
use chrono::Utc;
use serde_json::{Value, json};
use support::*;

async fn agent(app: &TestApp) -> (Api, String) {
    let (_, login) = app.new_user("SALES", PASSWORD).await;
    (app.as_user(&login).await, format!("Test {login}"))
}

async fn report(c: &Api, category: &str, sub: &str, fields: Value, title: &str) -> Value {
    let (requester, _) = person("Sprzedaż").await;
    let r = c
        .post(
            "/api/desk/tickets",
            json!({"title": title, "description": "Dane do raportu", "categoryKey": category,
                   "subcategoryKey": sub, "priority": "MEDIUM", "reportedAt": Utc::now().to_rfc3339(),
                   "requesterPersonId": requester, "fields": fields}),
        )
        .await;
    assert_eq!(r.status, 201, "{:?}", r.body);
    r.body
}

fn warsaw_today() -> chrono::NaiveDate {
    Utc::now()
        .with_timezone(&chrono_tz::Europe::Warsaw)
        .date_naive()
}

fn keys(table: &Value) -> Vec<String> {
    table["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["key"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn category_report_only_fields_with_a_value_and_exports_carry_the_same_table() {
    let app = TestApp::start().await;
    let (c, _) = agent(&app).await;
    let title = format!(
        "=HYPERLINK(\"x\") raport {}",
        &uuid::Uuid::new_v4().simple().to_string()[..8]
    );
    report(
        &c,
        "finance",
        "finance.payments",
        json!({"amount": "1234.56", "contractor": "Łódzka spółka"}),
        &title,
    )
    .await;
    let today = warsaw_today();
    let q = format!("from={today}&to={today}");

    let table = c
        .get(&format!("/api/desk/reports/categories/finanse?{q}"))
        .await;
    assert_eq!(table.status, 200, "{:?}", table.body);
    let table = table.body;
    let k = keys(&table);
    assert!(k.contains(&"field:amount".into()) && k.contains(&"field:contractor".into()));
    assert!(!k.contains(&"field:limosa-number".into()) && !k.contains(&"market".into()));
    let column = |key: &str| k.iter().position(|c| c == key).unwrap();
    let rows = table["rows"].as_array().unwrap();
    let row = rows
        .iter()
        .find(|r| r[column("title")] == title.as_str())
        .unwrap();
    assert_eq!(row[column("field:amount")], "1234.56");
    assert_eq!(row[column("status")], "W puli");
    assert_eq!(row[column("priority")], "Średni");
    assert_eq!(table["title"], "Zgłoszenie do Finansów");
    let kinds: Vec<_> = table["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds[column("reported")], "DATETIME");
    assert_eq!(kinds[column("field:amount")], "AMOUNT");

    let english = c
        .get(&format!("/api/desk/reports/categories/finanse?{q}&lang=en"))
        .await
        .body;
    assert_eq!(english["columns"][0]["label"], "Number");
    assert_ne!(english["title"], table["title"], "catalog names in English");

    // CSV: BOM, ';', CRLF, decimal comma, a formula neutralised.
    let (status, headers, bytes) = c
        .download(&format!(
            "/api/desk/reports/categories/finanse/export.csv?{q}"
        ))
        .await;
    assert_eq!(status, 200);
    assert_eq!(headers["content-type"], "text/csv; charset=UTF-8");
    assert_eq!(headers["x-content-type-options"], "nosniff");
    let disposition = headers["content-disposition"].to_str().unwrap();
    assert!(
        disposition.starts_with("attachment;")
            && disposition.contains(&format!("raport-finanse-{today}.csv")),
        "{disposition}"
    );
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.starts_with("\u{feff}Numer;Data zgłoszenia;"), "{text}");
    let lines: Vec<&str> = text.trim_end_matches("\r\n").split("\r\n").collect();
    assert_eq!(lines.len(), rows.len() + 1, "a header and one line per row");
    assert!(text.contains("\"'=HYPERLINK(\"\"x\"\")"), "{text}");
    assert!(text.contains(";1234,56"), "decimal comma");

    // XLSX: the same rows, real numbers and dates.
    let (status, headers, bytes) = c
        .download(&format!(
            "/api/desk/reports/categories/finanse/export.xlsx?{q}"
        ))
        .await;
    assert_eq!(status, 200);
    assert_eq!(
        headers["content-type"],
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    );
    let mut book: Xlsx<_> = calamine::open_workbook_from_rs(Cursor::new(bytes)).unwrap();
    let sheet = book.worksheet_range_at(0).unwrap().unwrap();
    assert_eq!(sheet.get((0, 0)), Some(&Data::String("Numer".into())));
    assert_eq!(sheet.height(), rows.len() + 1);
    let amounts: Vec<f64> = (1..sheet.height())
        .filter_map(|r| match sheet.get((r, column("field:amount"))) {
            Some(Data::Float(f)) => Some(*f),
            _ => None,
        })
        .collect();
    assert!(amounts.contains(&1234.56), "{amounts:?}");
    assert!(
        matches!(sheet.get((1, column("reported"))), Some(Data::DateTime(_))),
        "a real date: {:?}",
        sheet.get((1, column("reported")))
    );

    // PDF: Lato embedded, Polish letters come out as text.
    let (status, headers, bytes) = c
        .download(&format!(
            "/api/desk/reports/categories/finanse/export.pdf?{q}"
        ))
        .await;
    assert_eq!(status, 200);
    assert_eq!(headers["content-type"], "application/pdf");
    assert_eq!(&bytes[..4], b"%PDF");
    let pdf = pdf_extract::extract_text_from_mem(&bytes).unwrap();
    assert!(pdf.contains("Zgłoszenie do Finansów"), "{pdf}");
    assert!(pdf.contains("Łódzka spółka"), "{pdf}");

    assert_eq!(
        c.get(&format!("/api/desk/reports/categories/nope?{q}"))
            .await
            .status,
        404
    );
    assert_eq!(
        c.download(&format!(
            "/api/desk/reports/categories/finanse/export.docx?{q}"
        ))
        .await
        .0,
        404
    );
    let backwards = c
        .get(&format!(
            "/api/desk/reports/categories/finanse?from={today}&to={}",
            today.pred_opt().unwrap()
        ))
        .await;
    assert_eq!(backwards.status, 400);
    assert_eq!(backwards.body["errorNumber"], 51150);
    let too_long = c
        .get("/api/desk/reports/categories/finanse?from=2025-01-01&to=2026-01-03")
        .await;
    assert_eq!(too_long.body["errorNumber"], 51150);
    assert_eq!(
        c.get(&format!(
            "/api/desk/reports/categories/finanse?{q}&completion=MAYBE"
        ))
        .await
        .body["errorNumber"],
        51116
    );
    assert_eq!(
        c.get("/api/desk/reports/categories/finanse?to=2026-01-01")
            .await
            .status,
        400,
        "from is required"
    );
}

#[tokio::test]
async fn markets_filters_and_choices_in_english() {
    let app = TestApp::start().await;
    let (c, _) = agent(&app).await;
    let title = format!("Rynek {}", &uuid::Uuid::new_v4().simple().to_string()[..8]);
    report(
        &c,
        "marketing",
        "marketing.budget",
        json!({"channels": "tiktok, Facebook", "campaign-goal": "rekrutacja", "amount": "500"}),
        &title,
    )
    .await;
    let today = warsaw_today();
    let q = format!("from={today}&to={today}");
    let find = |table: &Value| -> Option<Value> {
        let k = keys(table);
        let t = k.iter().position(|c| c == "title").unwrap();
        table["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r[t] == title.as_str())
            .cloned()
    };
    let english = c
        .get(&format!(
            "/api/desk/reports/categories/marketing?{q}&lang=en"
        ))
        .await
        .body;
    let row = find(&english).unwrap();
    let k = keys(&english);
    let channels = &row[k.iter().position(|c| c == "field:channels").unwrap()];
    assert_eq!(channels, "Facebook, TikTok");
    let goal = &row[k.iter().position(|c| c == "field:campaign-goal").unwrap()];
    assert_ne!(goal, "Rekrutacja", "a choice is translated: {goal}");

    let done = c
        .get(&format!(
            "/api/desk/reports/categories/marketing?{q}&completion=DONE"
        ))
        .await
        .body;
    assert!(find(&done).is_none(), "the ticket is still open");
    let open = c
        .get(&format!(
            "/api/desk/reports/categories/marketing?{q}&completion=OPEN&subcategoryKey=marketing.budget"
        ))
        .await
        .body;
    assert!(find(&open).is_some());
    let other = c
        .get(&format!(
            "/api/desk/reports/categories/marketing?{q}&subcategoryKey=marketing.other"
        ))
        .await
        .body;
    assert!(find(&other).is_none());
}

#[tokio::test]
async fn agents_report_counts_taking_and_resolving_from_the_events() {
    let app = TestApp::start().await;
    let (c, name) = agent(&app).await;
    let row_of = |table: &Value| -> Option<Vec<Value>> {
        table["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r[0] == name.as_str())
            .map(|r| r.as_array().unwrap().clone())
    };
    let before = c.get("/api/desk/reports/agents").await.body;
    assert!(
        row_of(&before).is_none(),
        "a new agent has done nothing yet"
    );

    // IT has no category report: the finance report checked by another test of this file stays unchanged.
    let done = report(&c, "it", "it.hardware", json!({}), "Do raportu agentów").await;
    let open = report(&c, "it", "it.hardware", json!({}), "W realizacji").await;
    for (ticket, resolve) in [(&done, true), (&open, false)] {
        let id = ticket["id"].as_i64().unwrap();
        assert_eq!(
            c.post(&format!("/api/desk/tickets/{id}/take"), Value::Null)
                .await
                .status,
            200
        );
        if resolve {
            assert_eq!(
                c.post(&format!("/api/desk/tickets/{id}/resolve"), Value::Null)
                    .await
                    .status,
                200
            );
        }
    }

    let table = c.get("/api/desk/reports/agents").await.body;
    let row = row_of(&table).unwrap();
    // agent, then taken / resolved / late for week, month, 3 months, then in progress now.
    assert_eq!(row[1], "2", "taken this week");
    assert_eq!(row[2], "1", "resolved this week");
    assert_eq!(row[3], "0", "late this week");
    assert_eq!(row[7], "2", "taken in 3 months");
    assert_eq!(row[10], "1", "in progress now");
    assert_eq!(table["totals"][0], "Razem");
    assert_eq!(table["columns"].as_array().unwrap().len(), 11);
    let group = table["columns"][1]["group"].as_str().unwrap();
    assert!(group.starts_with("Tydzień "), "{group}");
    assert_eq!(table["subtitle"], "Bieżące okresy");
    let total: i64 = table["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r[1].as_str().unwrap().parse::<i64>().unwrap())
        .sum();
    assert_eq!(table["totals"][1], total.to_string());

    let english = c.get("/api/desk/reports/agents?lang=EN").await.body;
    assert_eq!(english["totals"][0], "Total");
    let previous = c.get("/api/desk/reports/agents?previous=true").await;
    assert_eq!(previous.status, 200);
    assert_eq!(previous.body["subtitle"], "Poprzednie okresy");
    assert!(row_of(&previous.body).is_none_or(|r| r[1] == "0"));

    for format in ["csv", "xlsx", "pdf"] {
        let (status, headers, bytes) = c
            .download(&format!("/api/desk/reports/agents/export.{format}"))
            .await;
        assert_eq!(status, 200, "{format}");
        assert!(
            headers["content-disposition"]
                .to_str()
                .unwrap()
                .contains(&format!("raport-agenci-{}.{format}", warsaw_today()))
        );
        assert!(!bytes.is_empty());
    }
    let (_, _, pdf) = c.download("/api/desk/reports/agents/export.pdf").await;
    let text = pdf_extract::extract_text_from_mem(&pdf).unwrap();
    assert!(
        text.contains("Praca agentów") && text.contains("Razem"),
        "{text}"
    );
}

#[tokio::test]
async fn reports_need_a_session() {
    let app = TestApp::start().await;
    let anonymous = app.client();
    assert_eq!(anonymous.get("/api/desk/reports/agents").await.status, 401);
    assert_eq!(
        anonymous
            .download("/api/desk/reports/agents/export.csv")
            .await
            .0,
        401
    );
    assert_eq!(anonymous.get("/api/desk/charts").await.status, 401);
}

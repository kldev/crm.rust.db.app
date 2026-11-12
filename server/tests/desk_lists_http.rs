//! Desk lists, counts and counters (DeskHttpTest.kt). Its own test binary: the counters cover every ticket, so no
//! other test may write tickets meanwhile.

mod support;

use std::collections::HashSet;

use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use support::*;

fn instant(v: &Value) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(v.as_str().unwrap())
        .unwrap()
        .with_timezone(&Utc)
}

async fn all_pages(c: &Api, query: &str) -> Vec<Value> {
    let mut all = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let url = format!(
            "/api/desk/tickets?{query}&size=3{}",
            cursor
                .as_ref()
                .map(|c| format!("&cursor={c}"))
                .unwrap_or_default()
        );
        let page = c.get(&url).await;
        assert_eq!(page.status, 200, "{:?}", page.body);
        all.extend(page.body["items"].as_array().unwrap().clone());
        cursor = page.body["nextCursor"].as_str().map(str::to_owned);
        if cursor.is_none() {
            return all;
        }
    }
}

#[tokio::test]
async fn pool_and_mine_open_by_deadline_then_resolved_keyset_pages_count_and_counters() {
    let app = TestApp::start().await;
    let (me, login) = app.new_user("SALES", PASSWORD).await;
    let c = app.as_user(&login).await;
    let tag = format!("Lst{}", &uuid::Uuid::new_v4().simple().to_string()[..6]);

    // Open: overdue, due soon, later; some taken by me; some resolved (one late, one on time).
    let mut tickets = Vec::new();
    for (priority, hours_back) in [
        ("CRITICAL", 4),
        ("CRITICAL", 2),
        ("HIGH", 1),
        ("MEDIUM", 0),
        ("LOW", 0),
        ("LOW", 5),
        ("HIGH", 9),
    ] {
        tickets.push(
            report_ticket(
                &c,
                priority,
                Utc::now() - Duration::hours(hours_back),
                &format!("{tag} {priority}"),
            )
            .await,
        );
    }
    for t in &tickets[3..7] {
        c.post(&format!("/api/desk/tickets/{}/take", t["id"]), Value::Null)
            .await;
    }
    for t in &tickets[5..7] {
        let r = c
            .post(
                &format!("/api/desk/tickets/{}/resolve", t["id"]),
                Value::Null,
            )
            .await;
        assert_eq!(r.status, 200);
    }

    let mine = c.get("/api/desk/tickets?scope=MINE&size=200").await.body["items"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(mine.len(), 2);
    assert!(
        mine.iter()
            .all(|t| t["assigneeUserId"] == me && t["status"] == "IN_PROGRESS")
    );
    let pool = c.get("/api/desk/tickets?scope=POOL&size=200").await.body["items"]
        .as_array()
        .unwrap()
        .clone();
    assert!(pool.iter().all(|t| t["status"] == "IN_POOL"));
    let deadlines: Vec<_> = pool.iter().map(|t| instant(&t["deadline"])).collect();
    let mut sorted = deadlines.clone();
    sorted.sort();
    assert_eq!(deadlines, sorted);

    // Every ticket exactly once over all pages, open ones first by deadline, then resolved newest first.
    let all = all_pages(&c, "scope=ALL").await;
    let total: i64 = superuser()
        .await
        .query_one("SELECT count(*) FROM crm.tickets", &[])
        .await
        .unwrap()
        .get(0);
    assert_eq!(all.len() as i64, total);
    let ids: HashSet<_> = all.iter().map(|t| t["id"].as_i64().unwrap()).collect();
    assert_eq!(ids.len(), all.len());
    let first_done = all.iter().position(|t| t["status"] == "DONE").unwrap();
    assert!(first_done > 0 && all[first_done..].iter().all(|t| t["status"] == "DONE"));
    let resolved: Vec<_> = all[first_done..]
        .iter()
        .map(|t| instant(&t["resolvedAt"]))
        .collect();
    let mut newest_first = resolved.clone();
    newest_first.sort_by(|a, b| b.cmp(a));
    assert_eq!(resolved, newest_first);

    assert_eq!(c.get("/api/desk/tickets/count").await.body["total"], total);
    let counters = c.get("/api/desk/tickets/counters").await.body;
    assert_eq!(counters["mine"], mine.len());
    assert_eq!(counters["pool"], pool.len());
    let overdue = all_pages(&c, "deadline=OVERDUE").await;
    assert_eq!(counters["overdue"], overdue.len());
    assert!(
        overdue
            .iter()
            .all(|t| instant(&t["deadline"]) <= Utc::now() && t["status"] != "DONE")
    );
    let soon = all_pages(&c, "deadline=DUE_SOON").await;
    assert_eq!(counters["dueSoon"], soon.len());

    // Search: a title fragment in any case, or a number.
    let by_title = all_pages(&c, &format!("search={}", tag.to_lowercase())).await;
    assert_eq!(by_title.len(), tickets.len());
    let number = tickets[3]["number"].as_str().unwrap();
    let by_number = all_pages(&c, &format!("search={}", number.to_lowercase())).await;
    assert_eq!(by_number.len(), 1);
    assert_eq!(by_number[0]["id"], tickets[3]["id"]);
    assert_eq!(
        c.get(&format!(
            "/api/desk/tickets/count?search={tag}&completion=DONE"
        ))
        .await
        .body["total"],
        2
    );
    assert_eq!(
        c.get(&format!(
            "/api/desk/tickets/count?search={tag}&completion=OPEN"
        ))
        .await
        .body["total"],
        5
    );
    assert!(
        all_pages(&c, "search=%25").await.is_empty(),
        "wildcards are literal"
    );

    assert_eq!(c.get("/api/desk/tickets?scope=NOPE").await.status, 400);
    assert_eq!(
        c.get("/api/desk/tickets?scope=NOPE").await.body["errorNumber"],
        51116
    );
    assert_eq!(c.get("/api/desk/tickets?cursor=abc").await.status, 400);
}

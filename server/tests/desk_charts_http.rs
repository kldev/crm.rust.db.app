//! The desk charts (ADR-0011): the numbers agree with each other and with the lists.

mod support;

use chrono::{Duration, Utc};
use serde_json::{Value, json};
use support::*;

fn sum(values: &Value) -> i64 {
    values
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap())
        .sum()
}

fn sum_of(list: &Value, f: impl Fn(&Value) -> i64) -> i64 {
    list.as_array().unwrap().iter().map(f).sum()
}

#[tokio::test]
async fn the_numbers_agree_heatmap_flow_queue_and_counters() {
    let app = TestApp::start().await;
    let (_, login) = app.new_user("SALES", PASSWORD).await;
    let c = app.as_user(&login).await;
    let (requester, _) = person("Kadry").await;
    let new_ticket = |priority: &'static str, reported: chrono::DateTime<Utc>| {
        let c = &c;
        async move {
            let r = c
                .post(
                    "/api/desk/tickets",
                    json!({"title": "Test wykresy", "description": "Opis", "categoryKey": "legalization",
                           "subcategoryKey": "legalization.pesel", "priority": priority,
                           "reportedAt": reported.to_rfc3339(), "requesterPersonId": requester}),
                )
                .await;
            assert_eq!(r.status, 201, "{:?}", r.body);
            r.body["id"].as_i64().unwrap()
        }
    };
    // Resolved at once, one open and overdue (reported 10 days ago, low = 7 days), one open in the pool now.
    let resolved = new_ticket("HIGH", Utc::now()).await;
    c.post(&format!("/api/desk/tickets/{resolved}/take"), Value::Null)
        .await;
    let r = c
        .post(
            &format!("/api/desk/tickets/{resolved}/resolve"),
            Value::Null,
        )
        .await;
    assert_eq!(r.status, 200, "{:?}", r.body);
    new_ticket("LOW", Utc::now() - Duration::days(10)).await;
    new_ticket("MEDIUM", Utc::now()).await;

    for (range, buckets) in [("DAYS_7", 7), ("DAYS_30", 30), ("WEEKS_12", 12)] {
        let data = c.get(&format!("/api/desk/charts?range={range}")).await;
        assert_eq!(data.status, 200, "{:?}", data.body);
        let data = data.body;
        assert_eq!(data["range"], range);
        let figures = &data["figures"];
        let flow = &data["flow"];
        assert_eq!(flow.as_array().unwrap().len(), buckets, "{range}");
        assert_eq!(data["from"], flow[0]["from"]);
        assert_eq!(data["to"], flow[buckets - 1]["to"]);
        assert_eq!(
            figures["reported"].as_i64().unwrap(),
            sum_of(flow, |p| p["reported"].as_i64().unwrap())
        );
        assert_eq!(
            figures["resolved"].as_i64().unwrap(),
            sum_of(flow, |p| p["resolved"].as_i64().unwrap())
        );
        assert_eq!(figures["openNow"], flow[buckets - 1]["openAtEnd"]);
        assert_eq!(
            figures["reported"].as_i64().unwrap(),
            sum_of(&data["heat"], sum)
        );
        assert_eq!(data["heat"].as_array().unwrap().len(), 7);
        assert_eq!(data["heat"][0].as_array().unwrap().len(), 24);
        assert_eq!(
            figures["reported"].as_i64().unwrap(),
            sum_of(&data["byCategory"], |b| sum(&b["values"]))
        );
        let queue = &data["queue"];
        assert_eq!(
            figures["openNow"].as_i64().unwrap(),
            sum_of(queue, |b| sum(&b["values"]))
        );
        assert_eq!(
            figures["overdueNow"].as_i64().unwrap(),
            sum_of(queue, |b| b["values"][2].as_i64().unwrap())
        );
        assert_eq!(
            figures["resolved"].as_i64().unwrap(),
            sum_of(&data["agents"], |b| sum(&b["values"]))
        );
        assert!(figures["overdueNow"].as_i64().unwrap() >= 1);
    }

    let counters = c.get("/api/desk/tickets/counters").await.body;
    let all = c.get("/api/desk/charts?range=DAYS_30").await.body;
    assert_eq!(counters["overdue"], all["figures"]["overdueNow"]);
    assert_eq!(counters["dueSoon"], all["figures"]["dueSoonNow"]);
    let open = c
        .get("/api/desk/tickets/count?scope=ALL&completion=OPEN")
        .await
        .body;
    assert_eq!(
        open["total"], all["figures"]["openNow"],
        "open now = the list"
    );

    let legalization = c
        .get("/api/desk/charts?range=DAYS_7&categoryKey=legalization")
        .await
        .body;
    assert_eq!(legalization["bySubcategory"], true);
    assert_eq!(legalization["categoryKey"], "legalization");
    assert!(
        legalization["byCategory"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b["key"] == "legalization.pesel")
    );
    let high = legalization["sla"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["priority"] == "HIGH")
        .unwrap();
    assert!(high["count"].as_i64().unwrap() >= 1);
    assert!(
        high["median"].as_f64().unwrap() < 1.0,
        "resolved at once: a sliver of the SLA"
    );
    let me = c.me().await.to_string();
    let agent = legalization["agents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["key"] == me.as_str())
        .unwrap();
    assert_eq!(agent["label"], format!("Test {login}"));
    assert_eq!(agent["values"], json!([1, 0]));

    assert_eq!(c.get("/api/desk/charts").await.body["range"], "DAYS_30");
    assert_eq!(c.get("/api/desk/charts?range=YEARS").await.status, 400);
}

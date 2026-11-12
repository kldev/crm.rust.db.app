//! Service desk (ADR-0007): a ticket is an event stream; the procedures decide every action (DeskHttpTest.kt).

mod support;

use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use support::*;

fn instant(v: &Value) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(v.as_str().unwrap())
        .unwrap()
        .with_timezone(&Utc)
}

async fn events(api: &Api, ticket: &Value) -> Vec<Value> {
    api.get(&format!("/api/desk/tickets/{}/events", ticket["id"]))
        .await
        .body
        .as_array()
        .unwrap()
        .clone()
}

fn data(event: &Value) -> Value {
    serde_json::from_str(event["data"].as_str().unwrap()).unwrap()
}

/// An agent with a display name we know.
async fn agent(app: &TestApp) -> (i64, String, Api) {
    let (id, login) = app.new_user("SALES", PASSWORD).await;
    let api = app.as_user(&login).await;
    let name = api.get("/api/auth/me").await.body["displayName"]
        .as_str()
        .unwrap()
        .to_owned();
    (id, name, api)
}

#[tokio::test]
async fn a_reported_ticket_starts_in_the_pool_with_its_deadline_from_the_sla_and_one_event() {
    let app = TestApp::start().await;
    let (_, name, c) = agent(&app).await;
    let ticket = report_ticket(&c, "HIGH", Utc::now() - Duration::hours(2), "Test laptop").await;
    assert_eq!(ticket["status"], "IN_POOL");
    assert_eq!(ticket["channel"], "AGENT");
    assert_eq!(ticket["registeredByName"], name.as_str());
    let number = ticket["number"].as_str().unwrap();
    assert!(number.starts_with("HAL-") && number.len() == 12, "{number}");
    assert!(
        ticket["deadline"].as_str().unwrap().ends_with('Z'),
        "instants carry Z"
    );
    assert_eq!(
        (instant(&ticket["deadline"]) - instant(&ticket["reportedAt"])).num_minutes(),
        8 * 60
    );

    let stream = events(&c, &ticket).await;
    assert_eq!(stream.len(), 1);
    assert_eq!(stream[0]["type"], "TicketReported");
    let d = data(&stream[0]);
    assert_eq!(d["priority"], "HIGH");
    assert!(d["requester"]["name"].as_str().unwrap().starts_with("Imię"));
    assert_eq!(d["requester"]["incognito"], false);
}

#[tokio::test]
async fn report_rules_required_fields_catalog_backdating_and_the_requester() {
    let app = TestApp::start().await;
    let (_, _, c) = agent(&app).await;
    let (person, _) = person("Sprzedaż").await;
    let post = |changes: Value| {
        let mut body = json!({"title": "Test", "description": "Opis", "categoryKey": "it",
            "subcategoryKey": "it.hardware", "priority": "LOW", "reportedAt": Utc::now().to_rfc3339(),
            "requesterIncognito": true});
        for (k, v) in changes.as_object().unwrap() {
            body[k] = v.clone();
        }
        let c = &c;
        async move { c.post("/api/desk/tickets", body).await }
    };
    assert_eq!(
        post(json!({"subcategoryKey": "legal.contracts"}))
            .await
            .body["errorNumber"],
        51105
    );
    assert_eq!(
        post(json!({"categoryKey": "nope"})).await.body["errorNumber"],
        51104
    );
    let future = (Utc::now() + Duration::minutes(10)).to_rfc3339();
    assert_eq!(
        post(json!({"reportedAt": future})).await.body["errorNumber"],
        51108
    );
    let old = (Utc::now() - Duration::days(31)).to_rfc3339();
    assert_eq!(
        post(json!({"reportedAt": old})).await.body["errorNumber"],
        51109
    );
    assert_eq!(
        post(json!({"requesterPersonId": person})).await.body["errorNumber"],
        51110
    );
    assert_eq!(
        post(json!({"requesterIncognito": false})).await.body["errorNumber"],
        51110
    );
    assert_eq!(
        post(json!({"requesterIncognito": false, "requesterPersonId": 999_999}))
            .await
            .body["errorNumber"],
        51411
    );
    assert_eq!(
        post(json!({"title": " "})).await.status,
        400,
        "request validation, before the database"
    );
    assert_eq!(post(json!({"priority": "URGENT"})).await.status, 400);
    assert_eq!(post(json!({})).await.status, 201);
}

#[tokio::test]
async fn take_hand_over_return_and_resolve_follow_the_rules_of_the_assignee() {
    let app = TestApp::start().await;
    let (anna_id, anna_name, anna) = agent(&app).await;
    let (piotr_id, piotr_name, piotr) = agent(&app).await;
    let ticket = report_ticket(&anna, "MEDIUM", Utc::now(), "Test flow").await;
    let t = format!("/api/desk/tickets/{}", ticket["id"]);

    assert_eq!(
        piotr.post(&format!("{t}/resolve"), Value::Null).await.body["errorNumber"],
        51912
    );
    let taken = anna.post(&format!("{t}/take"), Value::Null).await;
    assert_eq!(taken.body["status"], "IN_PROGRESS");
    let faster = piotr.post(&format!("{t}/take"), Value::Null).await;
    assert_eq!(faster.body["errorNumber"], 51910);
    assert!(
        faster.body["detail"].as_str().unwrap().contains(&anna_name),
        "says who was faster"
    );
    let not_mine = piotr.post(&format!("{t}/resolve"), Value::Null).await;
    assert_eq!(not_mine.status, 403);
    assert_eq!(not_mine.body["errorNumber"], 51312);
    assert_eq!(
        anna.post(&format!("{t}/hand-over"), json!({"toUserId": anna_id}))
            .await
            .body["errorNumber"],
        51114
    );
    assert_eq!(
        anna.post(&format!("{t}/hand-over"), json!({"toUserId": 999_999}))
            .await
            .body["errorNumber"],
        51412
    );

    let handed = anna
        .post(
            &format!("{t}/hand-over"),
            json!({"toUserId": piotr_id, "reason": "Urlop"}),
        )
        .await;
    assert_eq!(handed.status, 200, "{:?}", handed.body);
    assert_eq!(handed.body["assigneeName"], piotr_name.as_str());
    assert_eq!(
        handed.body["deadline"], ticket["deadline"],
        "handover keeps the deadline"
    );
    assert_eq!(
        anna.post(&format!("{t}/return"), json!({})).await.body["errorNumber"],
        51311
    );

    let returned = piotr
        .post(&format!("{t}/return"), json!({"reason": "Nie mój zakres"}))
        .await;
    assert_eq!(returned.body["status"], "IN_POOL");
    assert!(returned.body["assigneeUserId"].is_null());

    anna.post(&format!("{t}/take"), Value::Null).await;
    let resolved = anna.post(&format!("{t}/resolve"), Value::Null).await;
    assert_eq!(resolved.body["status"], "DONE");
    assert_eq!(
        resolved.body["assigneeName"],
        anna_name.as_str(),
        "a resolved ticket keeps who resolved it"
    );
    assert_eq!(resolved.body["resolvedLate"], false);
    assert_eq!(
        anna.post(&format!("{t}/take"), Value::Null).await.body["errorNumber"],
        51911
    );
    assert_eq!(
        anna.post(&format!("{t}/priority"), json!({"priority": "LOW"}))
            .await
            .body["errorNumber"],
        51911
    );

    let types: Vec<_> = events(&anna, &ticket)
        .await
        .iter()
        .map(|e| e["type"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        types,
        [
            "TicketReported",
            "TicketTaken",
            "TicketHandedOver",
            "TicketReturnedToPool",
            "TicketTaken",
            "TicketResolved"
        ]
    );
    assert_eq!(resolved.body["version"], 6);
    let handed_over = data(&events(&anna, &ticket).await[2]);
    assert_eq!(handed_over["to"]["id"], piotr_id);
    assert_eq!(handed_over["reason"], "Urlop");
}

#[tokio::test]
async fn three_agents_taking_one_ticket_at_once_exactly_one_wins() {
    let app = TestApp::start().await;
    let a = agent(&app).await.2;
    let b = agent(&app).await.2;
    let c = agent(&app).await.2;
    let ticket = report_ticket(&a, "MEDIUM", Utc::now(), "Test race").await;
    let path = format!("/api/desk/tickets/{}/take", ticket["id"]);
    let (ra, rb, rc) = tokio::join!(
        a.post(&path, Value::Null),
        b.post(&path, Value::Null),
        c.post(&path, Value::Null)
    );
    let mut statuses = [ra.status, rb.status, rc.status];
    statuses.sort();
    assert_eq!(statuses, [200, 409, 409]);
    assert_eq!(events(&a, &ticket).await.len(), 2);
}

#[tokio::test]
async fn resolving_after_the_deadline_is_late_for_good() {
    let app = TestApp::start().await;
    let c = agent(&app).await.2;
    let ticket = report_ticket(&c, "CRITICAL", Utc::now() - Duration::hours(4), "Test late").await;
    c.post(
        &format!("/api/desk/tickets/{}/take", ticket["id"]),
        Value::Null,
    )
    .await;
    let resolved = c
        .post(
            &format!("/api/desk/tickets/{}/resolve", ticket["id"]),
            Value::Null,
        )
        .await
        .body;
    assert_eq!(resolved["resolvedLate"], true);
    assert_eq!(
        data(events(&c, &ticket).await.last().unwrap())["late"],
        true
    );
}

#[tokio::test]
async fn priority_change_moves_the_deadline_from_the_report_date_and_the_same_priority_is_a_no_op()
{
    let app = TestApp::start().await;
    let c = agent(&app).await.2;
    let ticket = report_ticket(&c, "LOW", Utc::now() - Duration::hours(1), "Test priority").await;
    let t = format!("/api/desk/tickets/{}", ticket["id"]);
    let changed = c
        .post(
            &format!("{t}/priority"),
            json!({"priority": "CRITICAL", "reason": "Stoi produkcja"}),
        )
        .await
        .body;
    assert_eq!(
        (instant(&changed["deadline"]) - instant(&changed["reportedAt"])).num_minutes(),
        180
    );
    assert_eq!(changed["originalPriority"], "LOW");
    let same = c
        .post(&format!("{t}/priority"), json!({"priority": "CRITICAL"}))
        .await
        .body;
    assert_eq!(same["version"], changed["version"]);
    assert_eq!(events(&c, &ticket).await.len(), 2);
    let d = data(&events(&c, &ticket).await[1]);
    assert_eq!(
        (d["from"].as_str(), d["to"].as_str()),
        (Some("LOW"), Some("CRITICAL"))
    );
}

#[tokio::test]
async fn comments_any_state_limits_stored_as_events_and_shown_as_text() {
    let app = TestApp::start().await;
    let c = agent(&app).await.2;
    let ticket = report_ticket(&c, "MEDIUM", Utc::now(), "Test comments").await;
    let t = format!("/api/desk/tickets/{}", ticket["id"]);
    assert_eq!(
        c.post(&format!("{t}/comments"), json!({"text": ""}))
            .await
            .status,
        400
    );
    assert_eq!(
        c.post(&format!("{t}/comments"), json!({"text": "x".repeat(2001)}))
            .await
            .status,
        400
    );
    let commented = c
        .post(
            &format!("{t}/comments"),
            json!({"text": "<script>alert(1)</script>"}),
        )
        .await
        .body;
    assert_eq!(commented["commentCount"], 1);
    assert_eq!(
        data(events(&c, &ticket).await.last().unwrap())["text"],
        "<script>alert(1)</script>"
    );
    assert_eq!(
        c.post("/api/desk/tickets/999999/comments", json!({"text": "x"}))
            .await
            .body["errorNumber"],
        51410
    );
}

#[tokio::test]
async fn handler_must_come_from_the_department_of_the_category() {
    let app = TestApp::start().await;
    let c = agent(&app).await.2;
    let (requester, _) = person("Sprzedaż").await;
    let r = c
        .post(
            "/api/desk/tickets",
            json!({"title": "Test HR", "description": "Aneks", "categoryKey": "hr-poland",
                   "subcategoryKey": "hr-poland.pl.contract", "priority": "MEDIUM",
                   "reportedAt": Utc::now().to_rfc3339(), "requesterPersonId": requester}),
        )
        .await;
    let ticket = r.body;
    let t = format!("/api/desk/tickets/{}", ticket["id"]);
    assert_eq!(
        c.post(&format!("{t}/handler"), json!({"personId": requester}))
            .await
            .body["errorNumber"],
        51912,
        "take it first"
    );
    c.post(&format!("{t}/take"), Value::Null).await;
    let (hr, hr_name) = person("Kadry").await;
    assert_eq!(
        c.post(&format!("{t}/handler"), json!({"personId": requester}))
            .await
            .body["errorNumber"],
        51115
    );
    let assigned = c
        .post(
            &format!("{t}/handler"),
            json!({"personId": hr, "reason": "Ma dostęp"}),
        )
        .await
        .body;
    assert_eq!(assigned["handlerName"], hr_name.as_str());
    let again = c
        .post(&format!("{t}/handler"), json!({"personId": hr}))
        .await
        .body;
    assert_eq!(
        again["version"], assigned["version"],
        "the same handler is a no-op"
    );
    let cleared = c
        .post(&format!("{t}/handler"), json!({"personId": null}))
        .await
        .body;
    assert!(cleared["handlerPersonId"].is_null());
    assert_eq!(events(&c, &ticket).await.len(), 4);
    assert!(data(events(&c, &ticket).await.last().unwrap())["person"].is_null());
}

#[tokio::test]
async fn public_form_catalog_and_names_without_sign_in_a_ticket_into_the_pool_nothing_else() {
    let app = TestApp::start().await;
    let (person_id, _) = person("Sprzedaż").await;
    let anonymous = app.client();
    let catalog = anonymous.get("/api/public/desk/catalog").await.body;
    assert_eq!(catalog["categories"].as_array().unwrap().len(), 8);
    assert_eq!(catalog["priorities"].as_array().unwrap().len(), 4);
    assert_eq!(
        catalog["priorities"][0],
        json!({"priority": "CRITICAL", "slaMinutes": 180})
    );
    let people = anonymous.get("/api/public/desk/people").await.body;
    let me = people
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == person_id)
        .expect("listed");
    assert!(me.get("email").is_none(), "the public list has no contacts");
    assert_eq!(anonymous.get("/api/desk/tickets").await.status, 401);
    assert_eq!(anonymous.get("/api/desk/catalog").await.status, 401);

    anonymous.csrf().await;
    let form = reqwest::multipart::Form::new()
        .text("title", "Test z formularza")
        .text("description", "Drukarka")
        .text("categoryKey", "it")
        .text("subcategoryKey", "it.hardware")
        .text("priority", "LOW")
        .text("requesterIncognito", "true");
    let sent = anonymous.multipart("/api/public/desk/tickets", form).await;
    assert_eq!(sent.status, 201, "{:?}", sent.body);
    let number = sent.body["number"].as_str().unwrap().to_owned();
    assert!(sent.body["deadline"].is_string());

    let agent_api = agent(&app).await.2;
    let ticket = agent_api
        .get(&format!(
            "/api/desk/tickets/by-number/{}",
            number.to_lowercase()
        ))
        .await
        .body;
    assert_eq!(ticket["channel"], "SELF_SERVICE");
    assert_eq!(ticket["status"], "IN_POOL");
    assert_eq!(ticket["requesterIncognito"], true);
    assert!(ticket["registeredByName"].is_null());

    let bad = reqwest::multipart::Form::new().text("title", "Bez reszty");
    assert_eq!(
        anonymous
            .multipart("/api/public/desk/tickets", bad)
            .await
            .status,
        400
    );
}

#[tokio::test]
async fn the_public_form_takes_a_limited_number_of_tickets_per_minute() {
    let app = TestApp::start_with(|c| c.desk_public_reports_per_minute = 2).await;
    let anonymous = app.client();
    anonymous.csrf().await;
    let form = || {
        reqwest::multipart::Form::new()
            .text("title", "Test limit")
            .text("description", "x")
            .text("categoryKey", "it")
            .text("subcategoryKey", "it.hardware")
            .text("priority", "LOW")
            .text("requesterIncognito", "true")
    };
    assert_eq!(
        anonymous
            .multipart("/api/public/desk/tickets", form())
            .await
            .status,
        201
    );
    assert_eq!(
        anonymous
            .multipart("/api/public/desk/tickets", form())
            .await
            .status,
        201
    );
    let third = anonymous
        .multipart("/api/public/desk/tickets", form())
        .await;
    assert_eq!(third.status, 429);
    assert!(third.body["detail"].as_str().unwrap().contains("Too many"));
}

#[tokio::test]
async fn agents_and_people_lists() {
    let app = TestApp::start().await;
    let (_, name, c) = agent(&app).await;
    let (person_id, _) = person("Sprzedaż").await;
    let prefix = &name[..name.len().min(8)].to_lowercase();
    let agents = c
        .get(&format!("/api/desk/agents?search={prefix}"))
        .await
        .body;
    let items = agents["items"].as_array().unwrap();
    assert!(items.iter().any(|a| a["displayName"] == name.as_str()));
    assert!(items.iter().all(|a| {
        a["displayName"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .starts_with(prefix)
            || a["login"].as_str().unwrap().starts_with(prefix)
    }));
    let people = c.get("/api/desk/people").await.body;
    let p = people
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == person_id)
        .unwrap();
    assert!(p["email"].as_str().unwrap().ends_with("@company.test"));
}

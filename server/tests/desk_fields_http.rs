//! Report data of desk tickets (ADR-0010): every kind of field, the PESEL and IBAN rules, changes as events.

mod support;

use chrono::Utc;
use serde_json::{Value, json};
use support::*;

async fn report(c: &Api, category: &str, sub: &str, fields: Value) -> Response {
    let (requester, _) = person("Sprzedaż").await;
    c.post(
        "/api/desk/tickets",
        json!({"title": "Test pola", "description": "Dane zgłoszenia", "categoryKey": category,
               "subcategoryKey": sub, "priority": "MEDIUM", "reportedAt": Utc::now().to_rfc3339(),
               "requesterPersonId": requester, "fields": fields}),
    )
    .await
}

async fn agent(app: &TestApp) -> Api {
    let (_, login) = app.new_user("SALES", PASSWORD).await;
    app.as_user(&login).await
}

fn values(fields: &Value) -> Vec<(String, String)> {
    fields
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["key"].as_str().unwrap().to_owned(),
                f["value"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

#[tokio::test]
async fn the_catalog_lists_the_fields_of_each_subcategory() {
    let app = TestApp::start().await;
    let catalog = app.client().get("/api/public/desk/catalog").await.body;
    let marketing = catalog["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["key"] == "marketing")
        .unwrap();
    let budget = marketing["subcategories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["key"] == "marketing.budget")
        .unwrap();
    let fields = budget["fields"].as_array().unwrap();
    assert_eq!(fields[0]["key"], "channels");
    assert_eq!(fields[0]["kind"], "MULTI_CHOICE");
    assert_eq!(fields[0]["required"], true);
    assert_eq!(fields[0]["options"][0], "Facebook");
    let currency = fields.iter().find(|f| f["key"] == "currency").unwrap();
    assert_eq!(currency["favorites"], json!(["PLN", "EUR"]));
    let extensions = catalog["attachmentExtensions"].as_array().unwrap();
    assert!(extensions.contains(&json!(".png")) && !extensions.contains(&json!(".svg")));
}

#[tokio::test]
async fn every_kind_is_checked_and_stored_in_its_normal_form() {
    let app = TestApp::start().await;
    let c = agent(&app).await;
    let r = report(
        &c,
        "marketing",
        "marketing.budget",
        json!({"channels": "tiktok, Facebook", "campaign-goal": "rekrutacja", "amount": "1 234,5",
               "currency": "eur", "period-from": "2026-12-01", "headcount": "1 200",
               "related-ticket": "hal-1a2b3c4d", "position": "  spawacz  "}),
    )
    .await;
    assert_eq!(r.status, 201, "{:?}", r.body);
    let stored = values(
        &c.get(&format!("/api/desk/tickets/{}/fields", r.body["id"]))
            .await
            .body,
    );
    assert_eq!(
        stored,
        [
            ("channels", "Facebook, TikTok"),
            ("campaign-goal", "Rekrutacja"),
            ("amount", "1234.50"),
            ("currency", "EUR"),
            ("period-from", "2026-12-01"),
            ("position", "spawacz"),
            ("headcount", "1200"),
            ("related-ticket", "HAL-1A2B3C4D"),
        ]
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
    );
    let events = c
        .get(&format!("/api/desk/tickets/{}/events", r.body["id"]))
        .await
        .body;
    let reported: Value = serde_json::from_str(events[0]["data"].as_str().unwrap()).unwrap();
    assert_eq!(reported["fields"]["amount"], "1234.50");
}

#[tokio::test]
async fn each_rule_has_its_number() {
    let app = TestApp::start().await;
    let c = agent(&app).await;
    let base = json!({"channels": "Facebook", "amount": "10"});
    let with = |k: &str, v: Value| {
        let mut b = base.clone();
        b[k] = v;
        b
    };
    let cases = [
        (
            json!({"channels": "Facebook", "amount": "10", "nope": "x"}),
            51130,
        ),
        (json!({"amount": "10"}), 51131),
        (with("position", json!("x".repeat(201))), 51132),
        (with("period-from", json!("2026-02-30")), 51133),
        (with("period-from", json!("1.12.2026")), 51133),
        (with("amount", json!("12,345")), 51135),
        (with("amount", json!("abc")), 51135),
        (with("headcount", json!("0")), 51137),
        (with("headcount", json!("dużo")), 51137),
        (with("channels", json!("Facebook, Telegram")), 51138),
        (with("currency", json!("BTC")), 51139),
        (with("related-ticket", json!("HAL-123")), 51140),
    ];
    for (fields, number) in cases {
        let r = report(&c, "marketing", "marketing.budget", fields.clone()).await;
        assert_eq!(r.body["errorNumber"], number, "{fields}");
    }
    let email = report(&c, "hr-poland", "hr-poland.pl.onboarding",
        json!({"first-name": "Jan", "last-name": "Nowak", "department": "IT", "email": "jan@@firma.pl"})).await;
    assert_eq!(email.body["errorNumber"], 51134);
    let iban = report(
        &c,
        "finance",
        "finance.payments",
        json!({"bank-account": "PL61 1090 1014 0000 0712 1981 2875"}),
    )
    .await;
    assert_eq!(iban.body["errorNumber"], 51136);
}

#[tokio::test]
async fn identity_document_rules_and_the_pesel_check_digit() {
    let app = TestApp::start().await;
    let c = agent(&app).await;
    let person = |extra: Value| {
        let mut b = json!({"first-name": "Jan", "last-name": "Nowak", "department": "IT"});
        for (k, v) in extra.as_object().unwrap() {
            b[k] = v.clone();
        }
        b
    };
    let cases = [
        (json!({"identity-number": "44051401458"}), 51141),
        (
            json!({"identity-document": "Brak dokumentów", "identity-number": "X"}),
            51142,
        ),
        (json!({"identity-document": "PESEL"}), 51143),
        (
            json!({"identity-document": "PESEL", "identity-number": "44051401459"}),
            51144,
        ),
        (json!({"identity-document": "Dowód osobisty"}), 51145),
    ];
    for (extra, number) in cases {
        let r = report(
            &c,
            "hr-poland",
            "hr-poland.pl.onboarding",
            person(extra.clone()),
        )
        .await;
        assert_eq!(r.body["errorNumber"], number, "{extra}");
    }
    let ok = report(
        &c,
        "hr-poland",
        "hr-poland.pl.onboarding",
        person(json!({"identity-document": "PESEL", "identity-number": "44051401458"})),
    )
    .await;
    assert_eq!(ok.status, 201, "{:?}", ok.body);
    let id_card = report(
        &c,
        "hr-poland",
        "hr-poland.pl.onboarding",
        person(json!({"identity-document": "dowód osobisty", "identity-number": "abc 123456"})),
    )
    .await;
    let stored = values(
        &c.get(&format!("/api/desk/tickets/{}/fields", id_card.body["id"]))
            .await
            .body,
    );
    assert!(stored.contains(&("identity-number".into(), "ABC123456".into())));
    assert!(stored.contains(&("identity-document".into(), "Dowód osobisty".into())));
}

#[tokio::test]
async fn iban_is_stored_without_spaces_and_a_polish_number_gets_pl() {
    let app = TestApp::start().await;
    let c = agent(&app).await;
    for (input, stored) in [
        (
            "pl61 1090 1014 0000 0712 1981 2874",
            "PL61109010140000071219812874",
        ),
        ("61109010140000071219812874", "PL61109010140000071219812874"),
        ("DE89 3704 0044 0532 0130 00", "DE89370400440532013000"),
    ] {
        let r = report(
            &c,
            "finance",
            "finance.payments",
            json!({"bank-account": input}),
        )
        .await;
        assert_eq!(r.status, 201, "{input}: {:?}", r.body);
        let values = values(
            &c.get(&format!("/api/desk/tickets/{}/fields", r.body["id"]))
                .await
                .body,
        );
        assert_eq!(values, [("bank-account".to_owned(), stored.to_owned())]);
    }
}

#[tokio::test]
async fn corrections_list_only_the_changes_and_nothing_changed_stores_nothing() {
    let app = TestApp::start().await;
    let c = agent(&app).await;
    let t = report(
        &c,
        "finance",
        "finance.advance",
        json!({"employee": "Jan Nowak", "amount": "100"}),
    )
    .await
    .body;
    let path = format!("/api/desk/tickets/{}/fields", t["id"]);
    let changed = c
        .put(
            &path,
            json!({"values": {"employee": "Jan Nowak", "amount": "150", "currency": "PLN"}}),
        )
        .await;
    assert_eq!(changed.status, 200, "{:?}", changed.body);
    assert_eq!(changed.body["version"], 2);
    let same = c
        .put(
            &path,
            json!({"values": {"employee": "Jan Nowak", "amount": "150.00", "currency": "pln"}}),
        )
        .await;
    assert_eq!(same.body["version"], 2, "nothing changed, nothing stored");
    let cleared = c
        .put(
            &path,
            json!({"values": {"employee": "Jan Nowak", "amount": "150", "currency": ""}}),
        )
        .await;
    assert_eq!(cleared.body["version"], 3);

    let events = c
        .get(&format!("/api/desk/tickets/{}/events", t["id"]))
        .await
        .body;
    let first: Value = serde_json::from_str(events[1]["data"].as_str().unwrap()).unwrap();
    assert_eq!(events[1]["type"], "TicketFieldsUpdated");
    assert_eq!(
        first["changes"],
        json!([{"key": "amount", "label": "Kwota", "from": "100.00", "to": "150.00"},
               {"key": "currency", "label": "Waluta", "from": null, "to": "PLN"}])
    );
    let last: Value = serde_json::from_str(events[2]["data"].as_str().unwrap()).unwrap();
    assert_eq!(
        last["changes"],
        json!([{"key": "currency", "label": "Waluta", "from": "PLN", "to": null}])
    );
    assert_eq!(values(&c.get(&path).await.body).len(), 2);

    assert_eq!(
        c.put(&path, json!({})).await.status,
        400,
        "values are required"
    );
    let no_fields = report_ticket(&c, "LOW", Utc::now(), "Test bez pól").await;
    let r = c
        .put(
            &format!("/api/desk/tickets/{}/fields", no_fields["id"]),
            json!({"values": {}}),
        )
        .await;
    assert_eq!(r.body["errorNumber"], 51146);
}

#[tokio::test]
async fn campaigns_are_linked_to_the_ticket_they_were_ordered_for() {
    let app = TestApp::start().await;
    let c = agent(&app).await;
    let recruitment = report(
        &c,
        "recruitment",
        "recruitment.client",
        json!({"client": "Firma", "position": "Spawacz"}),
    )
    .await
    .body;
    let number = recruitment["number"].as_str().unwrap();
    for _ in 0..2 {
        let r = report(&c, "marketing", "marketing.budget",
            json!({"channels": "Facebook", "amount": "500", "related-ticket": number.to_lowercase()})).await;
        assert_eq!(r.status, 201, "{:?}", r.body);
    }
    let linked = c
        .get(&format!("/api/desk/tickets/{}/linked", recruitment["id"]))
        .await
        .body;
    assert_eq!(linked.as_array().unwrap().len(), 2);
    assert!(
        linked
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["subcategoryKey"] == "marketing.budget")
    );
}

#[tokio::test]
async fn the_public_form_takes_report_data_as_a_json_field() {
    let app = TestApp::start().await;
    let anonymous = app.client();
    anonymous.csrf().await;
    let form = |fields: &str| {
        reqwest::multipart::Form::new()
            .text("title", "Test")
            .text("description", "x")
            .text("categoryKey", "finance")
            .text("subcategoryKey", "finance.advance")
            .text("priority", "LOW")
            .text("requesterIncognito", "true")
            .text("fields", fields.to_owned())
    };
    assert_eq!(
        anonymous
            .multipart("/api/public/desk/tickets", form(r#"{"amount": "12,50"}"#))
            .await
            .status,
        201
    );
    assert_eq!(
        anonymous
            .multipart("/api/public/desk/tickets", form("[1]"))
            .await
            .status,
        400
    );
    let bad = anonymous
        .multipart("/api/public/desk/tickets", form(r#"{"amount": "x"}"#))
        .await;
    assert_eq!(bad.body["errorNumber"], 51135);
}

//! Clients over HTTP (ClientHttpTest.kt).

mod support;

use serde_json::{Value, json};
use support::*;

fn nip() -> String {
    format!("19{:08}", rand_u32() % 100_000_000)
}

fn rand_u32() -> u32 {
    uuid::Uuid::new_v4().as_u128() as u32
}

fn client_body(name: &str, nip: Option<&str>) -> Value {
    json!({
        "name": name, "nip": nip, "email": format!("test.{}@example.test", rand_u32() % 1_000_000),
        "phone": "+48 500 000 000", "city": "Poznań", "status": "LEAD", "note": "Test note"
    })
}

fn with(mut body: Value, key: &str, value: Value) -> Value {
    body[key] = value;
    body
}

async fn count_db(sql: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> i64 {
    superuser()
        .await
        .query_one(sql, params)
        .await
        .unwrap()
        .get(0)
}

#[tokio::test]
async fn create_read_update_with_row_version_and_stale_row_version_is_409() {
    let app = TestApp::start().await;
    let (_, sales) = app.new_user("SALES", PASSWORD).await;
    let c = app.as_user(&sales).await;
    let created = c
        .post("/api/clients", client_body("Test Klient", Some(&nip())))
        .await;
    assert_eq!(created.status, 201, "{:?}", created.body);
    let id = created.body["id"].as_i64().unwrap();
    assert_eq!(created.body["assignedUserLogin"], sales.as_str());

    let read = c.get(&format!("/api/clients/{id}")).await.body;
    let v1 = read["rowVersion"].as_str().unwrap().to_owned();
    let update = with(
        with(
            client_body("Test Renamed", Some(&nip())),
            "rowVersion",
            json!(v1),
        ),
        "assignedUserId",
        read["assignedUserId"].clone(),
    );
    let updated = c.put(&format!("/api/clients/{id}"), update.clone()).await;
    assert_eq!(updated.status, 200, "{:?}", updated.body);
    assert_eq!(updated.body["name"], "Test Renamed");
    assert_ne!(updated.body["rowVersion"], v1.as_str());

    let stale = c.put(&format!("/api/clients/{id}"), update).await;
    assert_eq!(stale.status, 409);
    assert_eq!(stale.body["errorNumber"], 51901);

    let missing = c
        .put(&format!("/api/clients/{id}"), client_body("X", None))
        .await;
    assert_eq!(missing.status, 400, "rowVersion is required");
}

#[tokio::test]
async fn update_with_identical_values_does_not_change_row_version() {
    let app = TestApp::start().await;
    let (_, sales) = app.new_user("SALES", PASSWORD).await;
    let c = app.as_user(&sales).await;
    let created = c
        .post("/api/clients", client_body("Test Same", Some(&nip())))
        .await
        .body;
    let same = json!({
        "name": created["name"], "nip": created["nip"], "email": created["email"], "phone": created["phone"],
        "city": created["city"], "status": created["status"], "assignedUserId": created["assignedUserId"],
        "note": created["note"], "rowVersion": created["rowVersion"]
    });
    let r = c
        .put(&format!("/api/clients/{}", created["id"]), same)
        .await;
    assert_eq!(r.status, 200, "{:?}", r.body);
    assert_eq!(r.body["rowVersion"], created["rowVersion"]);
}

#[tokio::test]
async fn rules_of_the_procedure_come_back_as_400_with_the_error_number() {
    let app = TestApp::start().await;
    let (_, sales) = app.new_user("SALES", PASSWORD).await;
    let c = app.as_user(&sales).await;
    let bad_nip = c
        .post("/api/clients", client_body("Test", Some("12345")))
        .await;
    assert_eq!(bad_nip.status, 400);
    assert_eq!(bad_nip.body["errorNumber"], 51011);
    // Request validation, before the database.
    let blank = c.post("/api/clients", client_body("", None)).await;
    assert_eq!(blank.status, 400);
    assert!(blank.body.get("errorNumber").is_none());
    // NIP with dashes and spaces is normalised by the procedure.
    let n = nip();
    let dashed = format!("{}-{}-{} {}", &n[0..3], &n[3..6], &n[6..8], &n[8..10]);
    let r = c
        .post("/api/clients", client_body("Test dashed", Some(&dashed)))
        .await;
    assert_eq!(r.body["nip"], n.as_str());
}

#[tokio::test]
async fn duplicate_nip_is_a_conflict() {
    let app = TestApp::start().await;
    let (_, manager) = app.new_user("MANAGER", PASSWORD).await;
    let c = app.as_user(&manager).await;
    let n = nip();
    assert_eq!(
        c.post("/api/clients", client_body("Test A", Some(&n)))
            .await
            .status,
        201
    );
    let dup = c
        .post("/api/clients", client_body("Test B", Some(&n)))
        .await;
    assert_eq!(dup.status, 409);
    assert_eq!(dup.body["errorNumber"], 2627);
}

#[tokio::test]
async fn sales_user_sees_only_own_clients_and_cannot_assign_to_others() {
    let app = TestApp::start().await;
    let (_, other_login) = app.new_user("SALES", PASSWORD).await;
    let other = app.as_user(&other_login).await;
    let foreign = other
        .post("/api/clients", client_body("Test foreign", None))
        .await
        .body["id"]
        .as_i64()
        .unwrap();

    let (me, sales_login) = app.new_user("SALES", PASSWORD).await;
    let sales = app.as_user(&sales_login).await;
    sales
        .post("/api/clients", client_body("Test mine", None))
        .await;
    assert_eq!(
        sales.get(&format!("/api/clients/{foreign}")).await.status,
        404
    );
    let page = sales.get("/api/clients?size=200").await.body;
    let items = page["items"].as_array().unwrap();
    assert!(!items.is_empty());
    assert!(items.iter().all(|i| i["assignedUserId"] == me));

    let assign_other = sales
        .post(
            "/api/clients",
            with(
                client_body("Test", None),
                "assignedUserId",
                json!(me + 100_000),
            ),
        )
        .await;
    assert_eq!(assign_other.status, 403);
    assert_eq!(assign_other.body["errorNumber"], 51303);
}

#[tokio::test]
async fn keyset_pages_do_not_overlap_newest_first_and_by_case_insensitive_name() {
    let app = TestApp::start().await;
    let (_, sales_login) = app.new_user("SALES", PASSWORD).await;
    let c = app.as_user(&sales_login).await;
    let tag = format!("q{}", &uuid::Uuid::new_v4().simple().to_string()[..6]);
    // Mixed case: the prefix and the order ignore case.
    for (i, name) in [
        "Ab", "aC", "AD", "ae", "Af", "aG", "Ah", "AI", "aj", "Ak", "al", "Am",
    ]
    .iter()
    .enumerate()
    {
        let full = if i % 2 == 0 {
            format!("{}{name}", tag.to_uppercase())
        } else {
            format!("{tag}{name}")
        };
        assert_eq!(
            c.post("/api/clients", client_body(&full, None))
                .await
                .status,
            201
        );
    }

    let first = c.get("/api/clients?size=5").await.body;
    let second = c
        .get(&format!(
            "/api/clients?size=5&cursor={}",
            first["nextCursor"].as_str().unwrap()
        ))
        .await
        .body;
    let (ids1, ids2) = (ids(&first), ids(&second));
    assert_eq!(ids2.len(), 5);
    assert!(ids1.iter().min() > ids2.iter().max(), "newest first");

    let by_name1 = c
        .get(&format!("/api/clients?namePrefix={tag}&size=5"))
        .await
        .body;
    let by_name2 = c
        .get(&format!(
            "/api/clients?namePrefix={}&size=5&cursor={}",
            tag.to_uppercase(),
            by_name1["nextCursor"].as_str().unwrap()
        ))
        .await
        .body;
    let rows: Vec<String> = [by_name1, by_name2]
        .iter()
        .flat_map(|p| p["items"].as_array().unwrap().clone())
        .map(|i| i["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(rows.len(), 10);
    assert!(rows.iter().all(|n| n.to_lowercase().starts_with(&tag)));
    let mut sorted = rows.clone();
    sorted.sort_by_key(|n| n.to_lowercase());
    assert_eq!(rows, sorted, "ordered by name regardless of case");

    // Wildcards typed by the user are literals.
    let wild = c.get("/api/clients?namePrefix=%25&size=5").await.body;
    assert!(ids(&wild).is_empty());
}

#[tokio::test]
async fn count_matches_the_filters_and_the_visibility_of_the_list() {
    let app = TestApp::start().await;
    let (_, manager_login) = app.new_user("MANAGER", PASSWORD).await;
    let manager = app.as_user(&manager_login).await;
    let tag = format!("Cnt{}", &uuid::Uuid::new_v4().simple().to_string()[..6]);
    for _ in 0..3 {
        manager.post("/api/clients", client_body(&tag, None)).await;
    }
    // Counted while other tests write: compare in one statement each, with a filter only this test uses.
    let by_name = manager
        .get(&format!("/api/clients/count?namePrefix={tag}"))
        .await
        .body["total"]
        .as_i64()
        .unwrap();
    assert_eq!(by_name, 3);
    let leads = manager
        .get(&format!(
            "/api/clients/count?namePrefix={tag}&city=Pozna%C5%84&status=LEAD"
        ))
        .await
        .body["total"]
        .as_i64()
        .unwrap();
    assert_eq!(leads, 3);
    assert_eq!(
        manager
            .get(&format!(
                "/api/clients/count?namePrefix={tag}&status=ACTIVE"
            ))
            .await
            .body["total"],
        0
    );

    let (me, sales_login) = app.new_user("SALES", PASSWORD).await;
    let sales = app.as_user(&sales_login).await;
    sales
        .post("/api/clients", client_body("Test own", None))
        .await;
    let own = count_db(
        "SELECT count(*) FROM crm.clients WHERE assigned_user_id = $1",
        &[&(me as i32)],
    )
    .await;
    assert_eq!(sales.get("/api/clients/count").await.body["total"], own);
    assert_eq!(
        sales.get("/api/clients/count?status=NOPE").await.status,
        400
    );
}

#[tokio::test]
async fn invalid_cursor_page_size_and_path_are_400() {
    let app = TestApp::start().await;
    let (_, manager) = app.new_user("MANAGER", PASSWORD).await;
    let c = app.as_user(&manager).await;
    assert_eq!(c.get("/api/clients?cursor=%21%21%21").await.status, 400);
    assert_eq!(c.get("/api/clients?size=500").await.status, 400);
    assert_eq!(c.get("/api/clients/abc").await.status, 400);
    assert_eq!(c.get("/api/clients?size=x").await.status, 400);
}

#[tokio::test]
async fn delete_sales_forbidden_client_with_projects_is_a_conflict_manager_deletes() {
    let app = TestApp::start().await;
    let team = app.team(1).await;
    let (sales_id, sales_login) = team.sales[0].clone();
    let sales = app.as_user(&sales_login).await;
    let own = sales
        .post("/api/clients", client_body("Test delete", None))
        .await
        .body;
    let r = sales
        .delete(&format!(
            "/api/clients/{}?rowVersion={}",
            own["id"],
            own["rowVersion"].as_str().unwrap()
        ))
        .await;
    assert_eq!(r.status, 403);
    assert_eq!(r.body["errorNumber"], 51301);

    let with_projects = new_client(&sales, "Test with project").await;
    new_project(
        &sales,
        with_projects["id"].as_i64().unwrap(),
        team.id,
        sales_id,
        today(),
    )
    .await;

    let manager = app.as_user(&team.manager.1).await;
    let rv = manager
        .get(&format!("/api/clients/{}", with_projects["id"]))
        .await
        .body["rowVersion"]
        .clone();
    let conflict = manager
        .delete(&format!(
            "/api/clients/{}?rowVersion={}",
            with_projects["id"],
            rv.as_str().unwrap()
        ))
        .await;
    assert_eq!(conflict.status, 409);
    assert_eq!(conflict.body["errorNumber"], 51904);

    let stale = manager
        .delete(&format!(
            "/api/clients/{}?rowVersion=0000000000000001",
            own["id"]
        ))
        .await;
    assert_eq!(stale.status, 409);
    assert_eq!(stale.body["errorNumber"], 51901);
    assert_eq!(
        manager
            .delete(&format!("/api/clients/{}?rowVersion=xyz", own["id"]))
            .await
            .status,
        400
    );

    let ok = manager
        .delete(&format!(
            "/api/clients/{}?rowVersion={}",
            own["id"],
            own["rowVersion"].as_str().unwrap()
        ))
        .await;
    assert_eq!(ok.status, 204);
    assert_eq!(
        manager
            .get(&format!("/api/clients/{}", own["id"]))
            .await
            .status,
        404
    );
}

#[tokio::test]
async fn manager_assigns_only_to_an_active_sales_user() {
    let app = TestApp::start().await;
    let (_, manager_login) = app.new_user("MANAGER", PASSWORD).await;
    let (other_manager, _) = app.new_user("MANAGER", PASSWORD).await;
    let (sales, _) = app.new_user("SALES", PASSWORD).await;
    let m = app.as_user(&manager_login).await;
    let r = m
        .post(
            "/api/clients",
            with(
                client_body("Test", None),
                "assignedUserId",
                json!(other_manager),
            ),
        )
        .await;
    assert_eq!(r.body["errorNumber"], 51017);
    let ok = m
        .post(
            "/api/clients",
            with(client_body("Test", None), "assignedUserId", json!(sales)),
        )
        .await;
    assert_eq!(ok.status, 201);
    let unassigned = m.post("/api/clients", client_body("Test", None)).await;
    assert!(unassigned.body["assignedUserId"].is_null());
}

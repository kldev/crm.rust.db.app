//! User administration over HTTP: paging, validation, conflicts, rowVersion.

mod support;

use serde_json::json;
use support::*;

#[tokio::test]
async fn users_are_paged_by_id_with_a_cursor() {
    let app = TestApp::start().await;
    for _ in 0..3 {
        app.new_user("SALES", "Start-Passw0rd").await;
    }
    let admin = app.admin().await;
    let first = admin.get("/api/users?size=2").await;
    assert_eq!(first.status, 200);
    let items = first.body["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    let cursor = first.body["nextCursor"].as_str().expect("next page");
    let second = admin
        .get(&format!("/api/users?size=2&cursor={cursor}"))
        .await;
    let next = second.body["items"].as_array().unwrap();
    assert!(next[0]["id"].as_i64() > items[1]["id"].as_i64());

    assert_eq!(admin.get("/api/users?size=0").await.status, 400);
    assert_eq!(admin.get("/api/users?cursor=@@@").await.status, 400);
    let only_sales = admin.get("/api/users?role=SALES&size=200").await;
    assert!(
        only_sales.body["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|u| u["role"] == "SALES")
    );
}

#[tokio::test]
async fn created_user_has_the_api_shape() {
    let app = TestApp::start().await;
    let login = format!("shape.{}", &uuid::Uuid::new_v4().simple().to_string()[..6]);
    let r = app
        .admin()
        .await
        .post("/api/users", json!({"login": login.to_uppercase(), "displayName": " Anna ", "role": "SALES", "password": "Start-Passw0rd"}))
        .await;
    assert_eq!(r.status, 201, "{:?}", r.body);
    let u = &r.body;
    assert_eq!(u["login"], login.as_str(), "login is stored in lower case");
    assert_eq!(u["displayName"], "Anna");
    assert_eq!(u["active"], true);
    assert!(u["lockedUntil"].is_null());
    let row_version = u["rowVersion"].as_str().unwrap();
    assert_eq!(row_version.len(), 16);
    assert!(row_version.chars().all(|c| c.is_ascii_hexdigit()));
    let created = u["createdAt"].as_str().unwrap();
    assert!(
        !created.ends_with('Z'),
        "LocalDateTime without zone: {created}"
    );
    assert!(
        created.parse::<chrono::NaiveDateTime>().is_ok(),
        "{created}"
    );
}

#[tokio::test]
async fn duplicate_login_is_409_and_invalid_body_is_400() {
    let app = TestApp::start().await;
    let (_, login) = app.new_user("SALES", "Start-Passw0rd").await;
    let admin = app.admin().await;
    let dup = admin
        .post("/api/users", json!({"login": login, "displayName": "X", "role": "SALES", "password": "Start-Passw0rd"}))
        .await;
    assert_eq!(dup.status, 409);
    assert_eq!(dup.body["errorNumber"], 2627);

    let bad = admin
        .post(
            "/api/users",
            json!({"login": "ab", "displayName": "", "role": "BOSS", "password": "short"}),
        )
        .await;
    assert_eq!(bad.status, 400);
    let detail = bad.body["detail"].as_str().unwrap();
    for field in ["login", "displayName", "role", "password"] {
        assert!(detail.contains(field), "{detail}");
    }
    // Characters outside the login rule pass the size check and are refused by the procedure.
    let rule = admin
        .post("/api/users", json!({"login": "bad login!", "displayName": "X", "role": "SALES", "password": "Start-Passw0rd"}))
        .await;
    assert_eq!(rule.status, 400);
    assert_eq!(rule.body["errorNumber"], 51015);
    assert_eq!(
        admin
            .post("/api/users", serde_json::Value::String("x".into()))
            .await
            .status,
        400
    );
}

#[tokio::test]
async fn same_role_keeps_the_row_version_and_a_change_moves_it() {
    let app = TestApp::start().await;
    let (id, _) = app.new_user("SALES", "Start-Passw0rd").await;
    let admin = app.admin().await;
    let before = admin
        .put(&format!("/api/users/{id}/role"), json!({"role": "SALES"}))
        .await;
    let same = admin
        .put(&format!("/api/users/{id}/role"), json!({"role": "SALES"}))
        .await;
    assert_eq!(before.body["rowVersion"], same.body["rowVersion"]);
    let changed = admin
        .put(&format!("/api/users/{id}/role"), json!({"role": "MANAGER"}))
        .await;
    assert_eq!(changed.body["role"], "MANAGER");
    assert_ne!(changed.body["rowVersion"], same.body["rowVersion"]);
    assert_eq!(
        admin
            .put("/api/users/999999/role", json!({"role": "SALES"}))
            .await
            .status,
        404
    );
}

#[tokio::test]
async fn admin_reset_clears_a_lockout() {
    let app = TestApp::start().await;
    let (id, login) = app.new_user("SALES", "Start-Passw0rd").await;
    let c = app.client();
    for _ in 0..MAX_FAILED {
        c.login(&login, "wrong").await;
    }
    let admin = app.admin().await;
    let r = admin
        .put(
            &format!("/api/users/{id}/password"),
            json!({"password": "Other-Passw0rd"}),
        )
        .await;
    assert_eq!(r.status, 204);
    assert_eq!(
        app.client().login(&login, "Other-Passw0rd").await.status,
        200
    );
}

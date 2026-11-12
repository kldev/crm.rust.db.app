//! Login, lockout, session, CSRF and roles over HTTP (AuthHttpTest.kt).

mod support;

use serde_json::json;
use support::*;

const START: &str = "Start-Passw0rd";

#[tokio::test]
async fn wrong_password_unknown_login_and_empty_body_are_rejected_without_detail() {
    let app = TestApp::start().await;
    let (_, login) = app.new_user("SALES", START).await;
    let c = app.client();
    let wrong = c.login(&login, "not-the-password").await;
    let unknown = c.login("nobody.here", "whatever-123").await;
    assert_eq!(wrong.status, 401);
    assert_eq!(unknown.status, 401);
    assert_eq!(wrong.body["detail"], unknown.body["detail"]);
    assert_eq!(wrong.body["detail"], "Invalid login or password.");
    assert_eq!(
        c.post("/api/auth/login", json!({"login": ""})).await.status,
        400
    );
}

#[tokio::test]
async fn account_is_locked_after_the_configured_number_of_failures_even_for_the_right_password() {
    let app = TestApp::start().await;
    let (_, login) = app.new_user("SALES", START).await;
    let c = app.client();
    for _ in 0..MAX_FAILED {
        assert_eq!(c.login(&login, "wrong-password").await.status, 401);
    }
    assert_eq!(c.login(&login, START).await.status, 401);
    let db = superuser().await;
    let row = db
        .query_one(
            "SELECT locked_until FROM crm.users WHERE login = $1",
            &[&login],
        )
        .await
        .unwrap();
    let locked: Option<chrono::DateTime<chrono::Utc>> = row.get(0);
    assert!(locked.is_some(), "locked_until must be set");
}

#[tokio::test]
async fn successful_login_resets_the_failure_counter() {
    let app = TestApp::start().await;
    let (_, login) = app.new_user("SALES", START).await;
    let c = app.client();
    for _ in 0..MAX_FAILED - 1 {
        c.login(&login, "wrong-password").await;
    }
    assert_eq!(c.login(&login, START).await.status, 200);
    let db = superuser().await;
    let count: i32 = db
        .query_one(
            "SELECT failed_login_count FROM crm.users WHERE login = $1",
            &[&login],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(count, 0);
}

#[tokio::test]
async fn deactivated_user_cannot_log_in() {
    let app = TestApp::start().await;
    let (id, login) = app.new_user("SALES", START).await;
    let r = app
        .admin()
        .await
        .put(&format!("/api/users/{id}/active"), json!({"active": false}))
        .await;
    assert_eq!(r.status, 200);
    assert_eq!(r.body["active"], false);
    assert_eq!(app.client().login(&login, START).await.status, 401);
}

#[tokio::test]
async fn dev_login_is_off_unless_crm_dev_login_is_set() {
    let app = TestApp::start().await;
    let c = app.client();
    assert_eq!(c.get("/api/auth/dev-login").await.body["enabled"], false);
    c.csrf().await;
    assert_eq!(
        c.post("/api/auth/dev-login", json!({"login": ADMIN_LOGIN}))
            .await
            .status,
        404
    );
    assert_eq!(c.get("/api/auth/me").await.status, 401);
}

#[tokio::test]
async fn api_requires_a_session() {
    let app = TestApp::start().await;
    assert_eq!(app.client().get("/api/users").await.status, 401);
    assert_eq!(app.client().get("/api/auth/me").await.status, 401);
}

#[tokio::test]
async fn writes_require_the_csrf_header() {
    let app = TestApp::start().await;
    let admin = app.admin().await;
    let without = admin
        .post_without_csrf(
            "/api/auth/password",
            json!({"currentPassword": "x", "newPassword": "Passw0rd-123"}),
        )
        .await;
    assert_eq!(without.status, 403);
    // A login without the cookie is a write too.
    let fresh = app.client();
    assert_eq!(
        fresh
            .post(
                "/api/auth/login",
                json!({"login": ADMIN_LOGIN, "password": ADMIN_PASSWORD})
            )
            .await
            .status,
        403
    );
}

#[tokio::test]
async fn csrf_cookie_is_readable_by_the_web_app_and_kept_once_set() {
    let app = TestApp::start().await;
    let c = app.client();
    let first = c.csrf().await;
    assert_eq!(first.status, 204);
    let set_cookie = first.headers["set-cookie"].to_str().unwrap().to_owned();
    assert!(set_cookie.starts_with("XSRF-TOKEN="), "{set_cookie}");
    assert!(!set_cookie.to_lowercase().contains("httponly"));
    let token = c.cookie("XSRF-TOKEN").unwrap();
    let second = c.csrf().await;
    assert!(second.headers.get("set-cookie").is_none());
    assert_eq!(c.cookie("XSRF-TOKEN").unwrap(), token);
}

#[tokio::test]
async fn session_id_changes_on_login_and_logout_ends_the_session() {
    let app = TestApp::start().await;
    let (_, login) = app.new_user("SALES", START).await;
    let c = app.client();
    c.csrf().await;
    c.get("/api/auth/me").await;
    let r = c.login(&login, START).await;
    assert_eq!(r.status, 200);
    let cookie = r.headers["set-cookie"].to_str().unwrap().to_owned();
    assert!(cookie.starts_with("JSESSIONID="), "{cookie}");
    let lower = cookie.to_lowercase();
    assert!(
        lower.contains("httponly") && lower.contains("samesite=lax"),
        "{cookie}"
    );
    let first_session = c.cookie("JSESSIONID").unwrap();

    // A second login in the same browser gets a new session id (fixation protection).
    c.login(&login, START).await;
    assert_ne!(c.cookie("JSESSIONID").unwrap(), first_session);

    assert_eq!(c.get("/api/auth/me").await.body["login"], login.as_str());
    assert_eq!(
        c.post("/api/auth/logout", serde_json::Value::Null)
            .await
            .status,
        204
    );
    assert_eq!(c.get("/api/auth/me").await.status, 401);
}

#[tokio::test]
async fn roles_are_enforced_by_the_endpoint_and_by_the_procedure() {
    let app = TestApp::start().await;
    let (_, sales) = app.new_user("SALES", START).await;
    let (_, manager) = app.new_user("MANAGER", START).await;
    let s = app.logged_in(&sales, START).await;
    assert_eq!(s.get("/api/users").await.status, 403);
    let m = app.logged_in(&manager, START).await;
    assert_eq!(m.get("/api/users").await.status, 200);
    assert_eq!(
        m.post("/api/users", json!({"login": "x.y.z", "displayName": "X", "role": "SALES", "password": "Passw0rd-123"}))
            .await
            .status,
        403
    );
}

#[tokio::test]
async fn admin_cannot_deactivate_or_demote_themselves() {
    let app = TestApp::start().await;
    let admin = app.admin().await;
    let me = admin.get("/api/auth/me").await.body["id"].as_i64().unwrap();
    let r = admin
        .put(&format!("/api/users/{me}/role"), json!({"role": "SALES"}))
        .await;
    assert_eq!(r.status, 400);
    assert_eq!(r.body["errorNumber"], 51018);
    let r = admin
        .put(&format!("/api/users/{me}/active"), json!({"active": false}))
        .await;
    assert_eq!(r.body["errorNumber"], 51018);
}

#[tokio::test]
async fn user_changes_own_password_with_the_current_one() {
    let app = TestApp::start().await;
    let (_, login) = app.new_user("SALES", START).await;
    let c = app.logged_in(&login, START).await;
    let bad = c
        .post(
            "/api/auth/password",
            json!({"currentPassword": "bad", "newPassword": "New-Passw0rd-1"}),
        )
        .await;
    assert_eq!(bad.status, 400);
    let good = c
        .post(
            "/api/auth/password",
            json!({"currentPassword": START, "newPassword": "New-Passw0rd-1"}),
        )
        .await;
    assert_eq!(good.status, 204);
    assert_eq!(app.client().login(&login, START).await.status, 401);
    assert_eq!(
        app.client().login(&login, "New-Passw0rd-1").await.status,
        200
    );
}

#[tokio::test]
async fn a_deactivated_actor_is_refused_by_the_procedures_with_51302() {
    let app = TestApp::start().await;
    let (id, login) = app.new_user("MANAGER", START).await;
    let m = app.logged_in(&login, START).await;
    app.admin()
        .await
        .put(&format!("/api/users/{id}/active"), json!({"active": false}))
        .await;
    // The session still holds the user; the procedure re-reads the active flag on every call.
    let r = m.get("/api/users").await;
    assert_eq!(r.status, 403);
    assert_eq!(r.body["errorNumber"], 51302);
}

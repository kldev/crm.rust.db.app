//! One-click development login (DevLoginHttpTest.kt).

mod support;

use serde_json::json;
use support::*;

#[tokio::test]
async fn dev_login_starts_a_session_for_an_active_user_without_a_password() {
    let app = TestApp::start_with(|c| c.auth.dev_login = true).await;
    let (_, manager) = app.new_user("MANAGER", "Start-Passw0rd").await;
    let c = app.client();
    assert_eq!(c.get("/api/auth/dev-login").await.body["enabled"], true);
    c.csrf().await;
    let r = c
        .post("/api/auth/dev-login", json!({"login": manager}))
        .await;
    assert_eq!(r.status, 200, "{:?}", r.body);
    assert_eq!(r.body["role"], "MANAGER");
    assert_eq!(c.get("/api/auth/me").await.body["login"], manager.as_str());
    // Roles still apply: a manager may read users but not create them.
    assert_eq!(
        c.post("/api/users", json!({"login": "x"})).await.status,
        403
    );
}

#[tokio::test]
async fn dev_login_rejects_an_unknown_login_and_still_needs_the_csrf_header() {
    let app = TestApp::start_with(|c| c.auth.dev_login = true).await;
    let c = app.client();
    c.csrf().await;
    assert_eq!(
        c.post("/api/auth/dev-login", json!({"login": "nobody.here"}))
            .await
            .status,
        401
    );
    assert_eq!(
        app.client()
            .post_without_csrf("/api/auth/dev-login", json!({"login": ADMIN_LOGIN}))
            .await
            .status,
        403
    );
}

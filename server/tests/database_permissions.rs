//! ADR-0013: the application login reaches schema `app` and nothing else (DatabasePermissionsTest.kt).

mod support;

use support::*;

async fn app_error(sql: &str) -> String {
    let client = app_login().await;
    let error = client.batch_execute(sql).await.expect_err(sql);
    error
        .as_db_error()
        .expect("database error")
        .code()
        .code()
        .to_owned()
}

#[tokio::test]
async fn application_login_cannot_read_or_write_a_table() {
    assert_eq!(app_error("SELECT id FROM crm.users LIMIT 1").await, "42501");
    assert_eq!(
        app_error("DELETE FROM crm.users WHERE id = -1").await,
        "42501"
    );
}

#[tokio::test]
async fn application_login_cannot_reach_internal_helpers_the_seed_or_the_history() {
    assert_eq!(app_error("SELECT crm.actor_require(1)").await, "42501");
    assert_eq!(app_error("SELECT 1 FROM seed.anything").await, "42501");
    assert_eq!(app_error("SELECT 1 FROM migration.history").await, "42501");
}

#[tokio::test]
async fn application_login_cannot_create_objects() {
    assert_eq!(app_error("CREATE TABLE app.x (id int)").await, "42501");
    assert_eq!(app_error("CREATE TABLE public.x (id int)").await, "42501");
}

#[tokio::test]
async fn application_login_can_use_schema_app() {
    let client = app_login().await;
    let ok: bool = client
        .query_one("SELECT ok FROM app.health_ping()", &[])
        .await
        .unwrap()
        .get(0);
    assert!(ok);
}

#[tokio::test]
async fn procedure_without_actor_is_a_host_error_u0001() {
    assert_eq!(
        app_error("CALL app.user_set_password(NULL, 1, 'x')").await,
        "U0001"
    );
}

#[tokio::test]
async fn migrations_are_idempotent() {
    assert_eq!(crm::migrate::migrate(&db_config()).await.unwrap(), 0);
}

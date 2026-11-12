//! Database access of the API (ADR-0013). The application login reaches only schema `app`: every write is
//! `CALL app.<procedure>(...)`, every read `SELECT ... FROM app.<function>(...)` or `app.v_<view>`. No ORM,
//! no query builder, no generated SQL: each repository function holds its one statement and maps the row by hand.

use std::time::Duration;

use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod, Runtime};
use tokio_postgres::{NoTls, Row, types::ToSql};

use crate::{config::DbConfig, error::ApiResult};

pub fn pg_config(db: &DbConfig, user: &str, password: &str) -> tokio_postgres::Config {
    let mut cfg = tokio_postgres::Config::new();
    cfg.host(&db.host)
        .port(db.port)
        .dbname(&db.name)
        .user(user)
        .password(password)
        .application_name("crm")
        .connect_timeout(db.connect_timeout);
    cfg
}

/// The pool of the application login.
pub fn app_pool(db: &DbConfig) -> anyhow::Result<Pool> {
    let manager = Manager::from_config(
        pg_config(db, &db.app_user, &db.app_password),
        NoTls,
        ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        },
    );
    Ok(Pool::builder(manager)
        .max_size(db.pool_size)
        .wait_timeout(Some(db.connect_timeout))
        .create_timeout(Some(db.connect_timeout))
        .recycle_timeout(Some(Duration::from_secs(5)))
        .runtime(Runtime::Tokio1)
        .build()?)
}

/// One statement on any client (pooled connection or transaction); the statement is prepared once per connection.
pub async fn query<C: deadpool_postgres::GenericClient + Sync>(
    client: &C,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> ApiResult<Vec<Row>> {
    let statement = client.prepare_cached(sql).await?;
    Ok(client.query(&statement, params).await?)
}

pub async fn query_one<C: deadpool_postgres::GenericClient + Sync>(
    client: &C,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> ApiResult<Row> {
    let statement = client.prepare_cached(sql).await?;
    Ok(client.query_one(&statement, params).await?)
}

pub async fn query_opt<C: deadpool_postgres::GenericClient + Sync>(
    client: &C,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> ApiResult<Option<Row>> {
    let statement = client.prepare_cached(sql).await?;
    Ok(client.query_opt(&statement, params).await?)
}

pub async fn execute<C: deadpool_postgres::GenericClient + Sync>(
    client: &C,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> ApiResult<()> {
    let statement = client.prepare_cached(sql).await?;
    client.execute(&statement, params).await?;
    Ok(())
}

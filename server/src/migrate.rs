//! CRM_MODE=init and CRM_MODE=migrate (ADR-0013). Not part of the API: this module is the only place that runs
//! SQL text other than `CALL app.*` / `SELECT ... FROM app.*` (the architecture test exempts it).
//!
//! - `init` (superuser): roles and the database (`db/init/*.sql`), idempotent.
//! - `migrate` (migrator login, as crm_owner): `db/migration/V<n>__*.sql` once and in order, then every
//!   `R__<schema>_*.sql` whose checksum changed (crm first, then app, then seed), each file in its own transaction,
//!   the whole run under an advisory lock, history in `migration.history`.

use std::time::Instant;

use anyhow::{Context, bail};
use include_dir::{Dir, include_dir};
use sha2::{Digest, Sha256};
use tokio_postgres::{Client, NoTls};

use crate::{config::DbConfig, db::pg_config};

static DB_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../db");

const LOCK_KEY: i64 = 0x0063_726d_6d69_6772; // "crmmigr"

async fn connect(cfg: tokio_postgres::Config) -> anyhow::Result<Client> {
    let (client, connection) = cfg.connect(NoTls).await.context("connect to PostgreSQL")?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!(error = %e, "migration connection closed");
        }
    });
    Ok(client)
}

fn literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn file(path: &str) -> anyhow::Result<&'static str> {
    DB_DIR
        .get_file(path)
        .and_then(|f| f.contents_utf8())
        .with_context(|| format!("missing db/{path}"))
}

fn fill(sql: &str, db: &DbConfig) -> String {
    sql.replace("${database}", &db.name)
        .replace("${migrator}", &db.migrator_user)
        .replace("${app_user}", &db.app_user)
        .replace("${migrator_password}", &literal(&db.migrator_password))
        .replace("${app_password}", &literal(&db.app_password))
}

/// Roles and the database. Superuser connection (CRM_DB_ADMIN_USER / CRM_DB_ADMIN_PASSWORD).
pub async fn init(db: &DbConfig) -> anyhow::Result<()> {
    if db.app_password.is_empty() || db.migrator_password.is_empty() {
        bail!("CRM_DB_APP_PASSWORD and CRM_DB_MIGRATOR_PASSWORD must be set");
    }
    let mut server = pg_config(db, &db.admin_user, &db.admin_password);
    server.dbname("postgres");
    let client = connect(server).await?;
    client
        .batch_execute(&fill(file("init/01_roles.sql")?, db))
        .await
        .context("db/init/01_roles.sql")?;
    let exists = client
        .query_opt("SELECT 1 FROM pg_database WHERE datname = $1", &[&db.name])
        .await?
        .is_some();
    if !exists {
        client
            .batch_execute(&format!(
                "CREATE DATABASE \"{}\" OWNER crm_owner ENCODING 'UTF8' TEMPLATE template0",
                db.name
            ))
            .await
            .context("create database")?;
        tracing::info!(database = %db.name, "database created");
    }
    drop(client);

    let client = connect(pg_config(db, &db.admin_user, &db.admin_password)).await?;
    client
        .batch_execute(&fill(file("init/02_database.sql")?, db))
        .await
        .context("db/init/02_database.sql")?;
    tracing::info!(database = %db.name, "init done");
    Ok(())
}

struct Script {
    name: &'static str,
    version: Option<i32>,
    sql: &'static str,
    checksum: String,
}

fn scripts() -> anyhow::Result<(Vec<Script>, Vec<Script>)> {
    let dir = DB_DIR
        .get_dir("migration")
        .context("missing db/migration")?;
    let mut versioned = Vec::new();
    let mut repeatable = Vec::new();
    for f in dir.files() {
        let name = f
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if !name.ends_with(".sql") {
            continue;
        }
        let sql = f
            .contents_utf8()
            .with_context(|| format!("{name} is not UTF-8"))?;
        let checksum = hex::encode(Sha256::digest(sql.as_bytes()));
        if let Some(rest) = name.strip_prefix('V') {
            let (number, _) = rest
                .split_once("__")
                .with_context(|| format!("{name}: expected V<n>__<name>.sql"))?;
            let version = number
                .parse()
                .with_context(|| format!("{name}: version must be a number"))?;
            versioned.push(Script {
                name,
                version: Some(version),
                sql,
                checksum,
            });
        } else if name.starts_with("R__") {
            repeatable.push(Script {
                name,
                version: None,
                sql,
                checksum,
            });
        } else {
            bail!("{name}: expected V<n>__*.sql or R__*.sql");
        }
    }
    versioned.sort_by_key(|s| s.version);
    for pair in versioned.windows(2) {
        if pair[0].version == pair[1].version {
            bail!("two migrations with version {:?}", pair[0].version);
        }
    }
    let rank = |name: &str| match name {
        n if n.starts_with("R__crm_") => 0,
        n if n.starts_with("R__app_") => 1,
        _ => 2,
    };
    repeatable.sort_by(|a, b| (rank(a.name), a.name).cmp(&(rank(b.name), b.name)));
    Ok((versioned, repeatable))
}

async fn apply(client: &mut Client, db: &DbConfig, script: &Script) -> anyhow::Result<()> {
    let started = Instant::now();
    let tx = client.transaction().await?;
    tx.batch_execute("SET LOCAL ROLE crm_owner").await?;
    tx.batch_execute(&fill(script.sql, db))
        .await
        .with_context(|| format!("migration {} failed", script.name))?;
    tx.execute(
        "INSERT INTO migration.history (script, version, checksum, duration_ms) VALUES ($1, $2, $3, $4)
         ON CONFLICT (script) DO UPDATE
             SET checksum = excluded.checksum, applied_at = now(), duration_ms = excluded.duration_ms",
        &[
            &script.name,
            &script.version,
            &script.checksum,
            &(started.elapsed().as_millis() as i32),
        ],
    )
    .await?;
    tx.commit().await?;
    tracing::info!(
        script = script.name,
        ms = started.elapsed().as_millis() as u64,
        "applied"
    );
    Ok(())
}

/// Applies pending migrations as the migrator login. Returns the number of scripts applied.
pub async fn migrate(db: &DbConfig) -> anyhow::Result<usize> {
    let (versioned, repeatable) = scripts()?;
    let mut client = connect(pg_config(db, &db.migrator_user, &db.migrator_password)).await?;
    client
        .execute("SELECT pg_advisory_lock($1)", &[&LOCK_KEY])
        .await?;

    let result = async {
        let applied: std::collections::HashMap<String, String> = client
            .query("SELECT script, checksum FROM migration.history", &[])
            .await
            .context("read migration.history (did CRM_MODE=init run?)")?
            .into_iter()
            .map(|r| (r.get(0), r.get(1)))
            .collect();

        let mut count = 0;
        for script in &versioned {
            match applied.get(script.name) {
                Some(sum) if *sum == script.checksum => {}
                Some(_) => bail!(
                    "{} changed after it was applied; add a new V migration instead",
                    script.name
                ),
                None => {
                    apply(&mut client, db, script).await?;
                    count += 1;
                }
            }
        }
        for script in &repeatable {
            if applied.get(script.name) != Some(&script.checksum) {
                apply(&mut client, db, script).await?;
                count += 1;
            }
        }
        Ok(count)
    }
    .await;

    client
        .execute("SELECT pg_advisory_unlock($1)", &[&LOCK_KEY])
        .await?;
    let count = result?;
    tracing::info!(applied = count, "migrations done");
    Ok(count)
}

#[cfg(test)]
mod tests {
    #[test]
    fn scripts_are_well_formed_and_ordered() {
        let (versioned, repeatable) = super::scripts().unwrap();
        assert_eq!(versioned[0].version, Some(1));
        assert!(
            repeatable
                .iter()
                .position(|s| s.name.starts_with("R__crm_"))
                .unwrap()
                < repeatable
                    .iter()
                    .position(|s| s.name.starts_with("R__app_"))
                    .unwrap()
        );
    }
}

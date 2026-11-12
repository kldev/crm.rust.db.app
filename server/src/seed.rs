//! CRM_MODE=seed (ADR-0004): the one data seed, run with the migrator login as crm_owner. The password hash of the
//! perf.* users is computed once here with the API's bcrypt (so k6 logs in through the real endpoint), then
//! `CALL seed.run(...)` generates every row in the database and `CALL seed.desk_run(...)` adds the service desk
//! (idempotent). CRM_SEED_DESK_ONLY=true runs only the desk part. Invalid settings stop with a non-zero exit code.
//! Like `migrate.rs`, not part of the API (the architecture test exempts it).

use std::time::Instant;

use anyhow::{Context, bail};
use futures_util::{StreamExt, stream};
use tokio_postgres::{AsyncMessage, Client, NoTls};

use crate::{config::DbConfig, db::pg_config};

/// The seed's settings; defaults are the performance data set (5M clients, 5M projects, 2M comments, 5000 users).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedConfig {
    pub clients: i32,
    pub admins: i32,
    pub managers: i32,
    pub sales: i32,
    pub team_size: i32,
    pub projects: i32,
    pub comments: i32,
    pub random_seed: i32,
    pub batch_size: i32,
    pub password: String,
    pub desk_tickets: i32,
    pub desk_only: bool,
}

impl SeedConfig {
    pub fn from_env() -> anyhow::Result<SeedConfig> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> anyhow::Result<SeedConfig> {
        let int = |name: &str, default: i32, min: i32, max: i32| -> anyhow::Result<i32> {
            let value = match get(name).filter(|v| !v.trim().is_empty()) {
                None => default,
                Some(v) => v
                    .trim()
                    .parse()
                    .with_context(|| format!("{name} must be a whole number, got '{v}'"))?,
            };
            if !(min..=max).contains(&value) {
                bail!("{name} must be between {min} and {max}, got {value}");
            }
            Ok(value)
        };
        let password = get("CRM_SEED_PASSWORD").unwrap_or_default();
        if password.chars().count() < 8 {
            bail!("CRM_SEED_PASSWORD must be set (at least 8 characters).");
        }
        let desk_only = match get("CRM_SEED_DESK_ONLY").as_deref().map(str::trim) {
            None | Some("") | Some("false") => false,
            Some("true") => true,
            Some(other) => bail!("CRM_SEED_DESK_ONLY must be true or false, got '{other}'"),
        };
        Ok(SeedConfig {
            clients: int("CRM_SEED_CLIENTS", 5_000_000, 0, 40_000_000)?,
            admins: int("CRM_SEED_ADMINS", 10, 0, 9_999)?,
            managers: int("CRM_SEED_MANAGERS", 490, 0, 9_999)?,
            sales: int("CRM_SEED_SALES", 4_500, 0, 9_999)?,
            team_size: int("CRM_SEED_TEAM_SIZE", 5, 1, 1_000)?,
            projects: int("CRM_SEED_PROJECTS", 5_000_000, 0, 40_000_000)?,
            comments: int("CRM_SEED_COMMENTS", 2_000_000, 0, 40_000_000)?,
            random_seed: int("CRM_SEED_RANDOM_SEED", 42, i32::MIN, i32::MAX)?,
            batch_size: int("CRM_SEED_BATCH_SIZE", 500_000, 1_000, 2_000_000)?,
            desk_tickets: int("CRM_SEED_DESK_TICKETS", 200, 0, 100_000)?,
            password,
            desk_only,
        })
    }
}

/// A migrator connection whose NOTICEs (the seed's progress) go to the log.
async fn connect(db: &DbConfig) -> anyhow::Result<Client> {
    let (client, mut connection) = pg_config(db, &db.migrator_user, &db.migrator_password)
        .connect(NoTls)
        .await
        .context("connect to PostgreSQL as the migrator")?;
    tokio::spawn(async move {
        let mut messages = stream::poll_fn(move |cx| connection.poll_message(cx));
        while let Some(message) = messages.next().await {
            match message {
                Ok(AsyncMessage::Notice(notice)) => tracing::info!("{}", notice.message()),
                Ok(_) => {}
                Err(e) => {
                    tracing::error!(error = %e, "seed connection closed");
                    break;
                }
            }
        }
    });
    client.batch_execute("SET ROLE crm_owner").await?;
    Ok(client)
}

/// Runs the seed; `password_hash` is computed once (bcrypt, the API's cost).
pub async fn run(db: &DbConfig, seed: &SeedConfig) -> anyhow::Result<()> {
    let password = seed.password.clone();
    let hash = tokio::task::spawn_blocking(move || crate::auth::hash_password_blocking(&password))
        .await??;
    let client = connect(db).await?;
    if !seed.desk_only {
        tracing::info!(
            clients = seed.clients,
            projects = seed.projects,
            comments = seed.comments,
            admins = seed.admins,
            managers = seed.managers,
            sales = seed.sales,
            team_size = seed.team_size,
            "seeding"
        );
        let row = client
            .query_one(
                "CALL seed.run($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                &[
                    &seed.clients,
                    &seed.admins,
                    &seed.managers,
                    &seed.sales,
                    &hash,
                    &seed.team_size,
                    &seed.projects,
                    &seed.comments,
                    &seed.random_seed,
                    &seed.batch_size,
                ],
            )
            .await
            .context("seed.run")?;
        tracing::info!(
            users = row.get::<_, i32>("users_created"),
            teams = row.get::<_, i32>("teams_created"),
            clients = row.get::<_, i32>("clients_created"),
            projects = row.get::<_, i32>("projects_created"),
            comments = row.get::<_, i32>("comments_created"),
            seconds = row.get::<_, i32>("seconds"),
            "seed finished"
        );
    }
    let started = Instant::now();
    client
        .execute(
            "CALL seed.desk_run($1, $2, $3)",
            &[&hash, &seed.desk_tickets, &seed.random_seed],
        )
        .await
        .context("seed.desk_run")?;
    tracing::info!(
        tickets = seed.desk_tickets,
        ms = started.elapsed().as_millis() as u64,
        "desk seed finished (tickets only into an empty desk)"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: std::collections::HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        move |name| map.get(name).cloned()
    }

    #[test]
    fn defaults_are_the_performance_data_set() {
        let c = SeedConfig::from_lookup(lookup(&[("CRM_SEED_PASSWORD", "Seed-Passw0rd")])).unwrap();
        assert_eq!(
            (
                c.clients,
                c.projects,
                c.comments,
                c.admins + c.managers + c.sales
            ),
            (5_000_000, 5_000_000, 2_000_000, 5_000)
        );
        assert_eq!(
            (c.batch_size, c.random_seed, c.desk_tickets, c.desk_only),
            (500_000, 42, 200, false)
        );
    }

    #[test]
    fn invalid_settings_stop_the_seed() {
        let err = |pairs: &[(&str, &str)]| {
            SeedConfig::from_lookup(lookup(pairs))
                .unwrap_err()
                .to_string()
        };
        assert!(err(&[]).contains("CRM_SEED_PASSWORD"));
        let ok = ("CRM_SEED_PASSWORD", "Seed-Passw0rd");
        assert!(err(&[ok, ("CRM_SEED_CLIENTS", "-1")]).contains("CRM_SEED_CLIENTS"));
        assert!(err(&[ok, ("CRM_SEED_BATCH_SIZE", "10")]).contains("CRM_SEED_BATCH_SIZE"));
        assert!(err(&[ok, ("CRM_SEED_SALES", "lots")]).contains("whole number"));
        assert!(err(&[ok, ("CRM_SEED_DESK_ONLY", "yes")]).contains("CRM_SEED_DESK_ONLY"));
    }
}

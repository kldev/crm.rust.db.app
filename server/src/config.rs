//! Settings from environment variables only (names as in infrastructure/.env-sample). Production-safe defaults;
//! every value is validated at start, so a wrong setting stops the process with a readable reason.

use std::{env, time::Duration};

use anyhow::{Context, bail};

use crate::storage::StorageConfig;

#[derive(Clone, Debug)]
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub name: String,
    pub app_user: String,
    pub app_password: String,
    pub migrator_user: String,
    pub migrator_password: String,
    /// Superuser for CRM_MODE=init (creates the database and the roles).
    pub admin_user: String,
    pub admin_password: String,
    pub pool_size: usize,
    pub connect_timeout: Duration,
}

#[derive(Clone, Debug)]
pub struct AuthConfig {
    pub max_failed_logins: i32,
    pub lock_minutes: i32,
    pub admin_login: Option<String>,
    pub admin_password: Option<String>,
    /// CRM_DEV_LOGIN=true: one-click login as any active user without a password (local only).
    pub dev_login: bool,
    pub session_timeout: Duration,
    pub cookie_secure: bool,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub db: DbConfig,
    pub auth: AuthConfig,
    /// The built React app (frontend/dist); missing directory = API only.
    pub static_dir: String,
    pub swagger_ui: bool,
    /// CRM_DESK_PUBLIC_REPORTS_PER_MINUTE: tickets the public form accepts per minute (whole application).
    pub desk_public_reports_per_minute: usize,
    /// The attachments bucket (ADR-0009): CRM_S3_ENDPOINT, CRM_S3_REGION, CRM_S3_ACCESS_KEY, CRM_S3_SECRET_KEY,
    /// CRM_S3_BUCKET.
    pub storage: StorageConfig,
}

fn var(name: &str) -> Option<String> {
    env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn var_or(name: &str, default: &str) -> String {
    var(name).unwrap_or_else(|| default.to_owned())
}

fn parse<T: std::str::FromStr>(name: &str, default: T) -> anyhow::Result<T>
where
    T::Err: std::fmt::Display,
{
    match var(name) {
        None => Ok(default),
        Some(raw) => raw
            .trim()
            .parse()
            .map_err(|e| anyhow::anyhow!("{name}={raw:?} is not valid: {e}")),
    }
}

fn int_in(name: &str, default: i32, min: i32, max: i32) -> anyhow::Result<i32> {
    let value = parse(name, default)?;
    if !(min..=max).contains(&value) {
        bail!("{name}={value} must be between {min} and {max}");
    }
    Ok(value)
}

fn flag(name: &str, default: bool) -> anyhow::Result<bool> {
    match var(name).map(|v| v.to_ascii_lowercase()) {
        None => Ok(default),
        Some(v) if v == "true" || v == "1" => Ok(true),
        Some(v) if v == "false" || v == "0" => Ok(false),
        Some(v) => bail!("{name}={v:?} must be true or false"),
    }
}

/// "30m", "1h", "45s", "90" (seconds) - the forms CRM_SESSION_TIMEOUT took with Spring.
pub fn parse_duration(name: &str, raw: &str) -> anyhow::Result<Duration> {
    let raw = raw.trim();
    let (number, unit) = raw
        .find(|c: char| !c.is_ascii_digit())
        .map(|i| raw.split_at(i))
        .unwrap_or((raw, "s"));
    let n: u64 = number
        .parse()
        .with_context(|| format!("{name}={raw:?} must look like 30m, 1h or 45s"))?;
    let seconds = match unit {
        "s" => n,
        "m" => n * 60,
        "h" => n * 3600,
        "d" => n * 86_400,
        _ => bail!("{name}={raw:?}: unit must be s, m, h or d"),
    };
    if seconds == 0 {
        bail!("{name} must be greater than zero");
    }
    Ok(Duration::from_secs(seconds))
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let pool_size = int_in("CRM_DB_POOL_SIZE", 30, 1, 500)? as usize;
        let db = DbConfig {
            host: var_or("CRM_DB_HOST", "localhost"),
            port: parse("CRM_DB_PORT", 5432u16)?,
            name: var_or("CRM_DB_NAME", "crm"),
            app_user: var_or("CRM_DB_APP_USER", "crm_app"),
            app_password: var_or("CRM_DB_APP_PASSWORD", ""),
            migrator_user: var_or("CRM_DB_MIGRATOR_USER", "crm_migrator"),
            migrator_password: var_or("CRM_DB_MIGRATOR_PASSWORD", ""),
            admin_user: var_or("CRM_DB_ADMIN_USER", "postgres"),
            admin_password: var_or("CRM_DB_ADMIN_PASSWORD", ""),
            pool_size,
            connect_timeout: Duration::from_millis(int_in(
                "CRM_DB_CONNECTION_TIMEOUT_MS",
                5000,
                100,
                120_000,
            )? as u64),
        };
        for (name, value) in [
            ("CRM_DB_NAME", &db.name),
            ("CRM_DB_APP_USER", &db.app_user),
            ("CRM_DB_MIGRATOR_USER", &db.migrator_user),
        ] {
            if !value
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            {
                bail!("{name}={value:?} may contain only a-z, 0-9 and _");
            }
        }

        let auth = AuthConfig {
            max_failed_logins: int_in("CRM_LOGIN_MAX_FAILED", 5, 1, 100)?,
            lock_minutes: int_in("CRM_LOGIN_LOCK_MINUTES", 15, 1, 1440)?,
            admin_login: var("CRM_ADMIN_LOGIN"),
            admin_password: var("CRM_ADMIN_PASSWORD"),
            dev_login: flag("CRM_DEV_LOGIN", false)?,
            session_timeout: parse_duration(
                "CRM_SESSION_TIMEOUT",
                &var_or("CRM_SESSION_TIMEOUT", "30m"),
            )?,
            cookie_secure: flag("CRM_COOKIE_SECURE", false)?,
        };

        Ok(Config {
            port: parse("PORT", 8080u16)?,
            db,
            auth,
            static_dir: var_or("CRM_STATIC_DIR", "static"),
            swagger_ui: flag("CRM_SWAGGER_UI", true)?,
            desk_public_reports_per_minute: int_in(
                "CRM_DESK_PUBLIC_REPORTS_PER_MINUTE",
                30,
                1,
                10_000,
            )? as usize,
            storage: StorageConfig {
                endpoint: var_or("CRM_S3_ENDPOINT", "http://localhost:9000"),
                region: var_or("CRM_S3_REGION", "us-east-1"),
                access_key: var_or("CRM_S3_ACCESS_KEY", ""),
                secret_key: var_or("CRM_S3_SECRET_KEY", ""),
                bucket: var_or("CRM_S3_BUCKET", "crm-attachments"),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::parse_duration;

    #[test]
    fn durations() {
        assert_eq!(parse_duration("X", "30m").unwrap().as_secs(), 1800);
        assert_eq!(parse_duration("X", "2h").unwrap().as_secs(), 7200);
        assert_eq!(parse_duration("X", "45").unwrap().as_secs(), 45);
        assert!(parse_duration("X", "0s").is_err());
        assert!(parse_duration("X", "5w").is_err());
    }
}

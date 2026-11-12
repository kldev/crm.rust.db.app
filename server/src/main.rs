//! One binary, four modes (CRM_MODE): app (default, HTTP API), init (roles and database, superuser),
//! migrate (db/migration, then exit), seed (test data, then exit).

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let json = std::env::var("CRM_LOG_FORMAT").is_ok_and(|v| v == "json");
    let filter = EnvFilter::try_from_env("CRM_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    if json {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .json()
            .init();
    } else {
        // Colours only on a terminal; container logs stay plain text.
        let ansi = std::io::IsTerminal::is_terminal(&std::io::stdout());
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(ansi)
            .init();
    }

    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!("{error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn run() -> anyhow::Result<()> {
    let config = crm::config::Config::from_env()?;
    match std::env::var("CRM_MODE")
        .unwrap_or_else(|_| "app".into())
        .as_str()
    {
        "app" => crm::serve(config).await,
        "init" => crm::migrate::init(&config.db).await,
        "migrate" => crm::migrate::migrate(&config.db).await.map(|_| ()),
        "seed" => {
            let seed = crm::seed::SeedConfig::from_env()?;
            crm::seed::run(&config.db, &seed).await
        }
        other => anyhow::bail!("unknown CRM_MODE '{other}' (expected: app, init, migrate, seed)"),
    }
}

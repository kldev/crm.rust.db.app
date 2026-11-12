//! Integration test support: one PostgreSQL 18 container per test binary (the image of the compose stack),
//! initialised and migrated by the same code as CRM_MODE=init / migrate, and the real HTTP app on a free port.
#![allow(dead_code)]

use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

use crm::{
    AppState,
    auth::session::MemorySessions,
    config::{AuthConfig, Config, DbConfig},
};
use reqwest::{Method, cookie::CookieStore};
use serde_json::Value;
use testcontainers::{
    GenericImage, ImageExt,
    core::{IntoContainerPort, WaitFor},
    runners::SyncRunner,
};

pub const PG_IMAGE: (&str, &str) = ("postgres", "18.6");
pub const ADMIN_LOGIN: &str = "admin";
pub const ADMIN_PASSWORD: &str = "Admin-Passw0rd";
pub const MAX_FAILED: i32 = 3;

struct Database {
    container: std::sync::Mutex<Option<testcontainers::Container<GenericImage>>>,
    config: DbConfig,
}

static DB: OnceLock<Database> = OnceLock::new();

/// Statics are never dropped in Rust: remove the container when the test process exits.
extern "C" fn remove_container() {
    if let Some(db) = DB.get()
        && let Ok(mut container) = db.container.lock()
    {
        drop(container.take());
    }
}

fn database() -> &'static Database {
    // Started on its own thread: the blocking runner cannot run inside the test's tokio runtime.
    DB.get_or_init(|| {
        let db = std::thread::spawn(start_database).join().unwrap();
        unsafe { libc::atexit(remove_container) };
        db
    })
}

fn start_database() -> Database {
    {
        let container = GenericImage::new(PG_IMAGE.0, PG_IMAGE.1)
            .with_exposed_port(5432.tcp())
            .with_wait_for(WaitFor::message_on_stderr(
                "database system is ready to accept connections",
            ))
            .with_env_var("POSTGRES_PASSWORD", "postgres")
            .start()
            .expect("start postgres:18");
        let port = container.get_host_port_ipv4(5432).unwrap();
        let config = DbConfig {
            host: "127.0.0.1".into(),
            port,
            name: "crm".into(),
            app_user: "crm_app".into(),
            app_password: "app-pass".into(),
            migrator_user: "crm_migrator".into(),
            migrator_password: "migrator-pass".into(),
            admin_user: "postgres".into(),
            admin_password: "postgres".into(),
            pool_size: 10,
            connect_timeout: Duration::from_secs(10),
        };
        let cfg = config.clone();
        {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                // The server restarts once after initdb; retry until it takes connections.
                for attempt in 0.. {
                    match crm::migrate::init(&cfg).await {
                        Ok(()) => break,
                        Err(e) if attempt < 30 => {
                            let _ = e;
                            tokio::time::sleep(Duration::from_millis(500)).await
                        }
                        Err(e) => panic!("init: {e:#}"),
                    }
                }
                crm::migrate::migrate(&cfg).await.expect("migrate");
            })
        }
        Database {
            container: std::sync::Mutex::new(Some(container)),
            config,
        }
    }
}

pub const S3_ACCESS_KEY: &str = "crmtestkey";
pub const S3_SECRET_KEY: &str = "crm-test-secret-key-0123456789";
pub const S3_BUCKET: &str = "crm-attachments";

struct RustFs {
    container: std::sync::Mutex<Option<testcontainers::Container<GenericImage>>>,
    endpoint: String,
}

static RUSTFS: OnceLock<RustFs> = OnceLock::new();

extern "C" fn remove_rustfs() {
    if let Some(fs) = RUSTFS.get()
        && let Ok(mut container) = fs.container.lock()
    {
        drop(container.take());
    }
}

/// One RustFS container per test binary - the image of docker compose - with the attachments bucket.
fn rustfs() -> &'static RustFs {
    RUSTFS.get_or_init(|| {
        let fs = std::thread::spawn(|| {
            let container = GenericImage::new("rustfs/rustfs", "1.0.0")
                .with_exposed_port(9000.tcp())
                .with_env_var("RUSTFS_ACCESS_KEY", S3_ACCESS_KEY)
                .with_env_var("RUSTFS_SECRET_KEY", S3_SECRET_KEY)
                .with_env_var("RUSTFS_VOLUMES", "/data")
                .with_env_var("RUSTFS_ADDRESS", ":9000")
                .start()
                .expect("start rustfs");
            let endpoint = format!(
                "http://127.0.0.1:{}",
                container.get_host_port_ipv4(9000).unwrap()
            );
            let client = s3_client(&endpoint);
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                for attempt in 0.. {
                    match client.create_bucket().bucket(S3_BUCKET).send().await {
                        Ok(_) => break,
                        Err(e) if attempt < 120 => {
                            let _ = e;
                            tokio::time::sleep(Duration::from_millis(500)).await
                        }
                        Err(e) => panic!("create bucket: {e:?}"),
                    }
                }
            });
            RustFs {
                container: std::sync::Mutex::new(Some(container)),
                endpoint,
            }
        })
        .join()
        .unwrap();
        unsafe { libc::atexit(remove_rustfs) };
        fs
    })
}

fn s3_client(endpoint: &str) -> aws_sdk_s3::Client {
    use aws_sdk_s3::config::{Builder, Credentials, Region, RequestChecksumCalculation};
    aws_sdk_s3::Client::from_conf(
        Builder::new()
            .behavior_version_latest()
            .endpoint_url(endpoint)
            .region(Region::new("us-east-1"))
            .force_path_style(true)
            .request_checksum_calculation(RequestChecksumCalculation::WhenRequired)
            .credentials_provider(Credentials::new(
                S3_ACCESS_KEY,
                S3_SECRET_KEY,
                None,
                None,
                "test",
            ))
            .build(),
    )
}

/// Keys of every object in the bucket (tests check that refused uploads are deleted again).
pub async fn bucket_keys() -> std::collections::BTreeSet<String> {
    let client = s3_client(&rustfs().endpoint);
    let mut keys = std::collections::BTreeSet::new();
    let mut token = None;
    loop {
        let page = client
            .list_objects_v2()
            .bucket(S3_BUCKET)
            .set_continuation_token(token)
            .send()
            .await
            .unwrap();
        keys.extend(
            page.contents()
                .iter()
                .filter_map(|o| o.key().map(str::to_owned)),
        );
        token = page.next_continuation_token().map(str::to_owned);
        if token.is_none() {
            return keys;
        }
    }
}

pub fn db_config() -> DbConfig {
    database().config.clone()
}

/// A superuser connection for checks of stored state (never the path the application uses).
pub async fn superuser() -> tokio_postgres::Client {
    let db = db_config();
    connect(&db.admin_user, &db.admin_password).await
}

/// A connection as the application login (crm_app), for permission tests.
pub async fn app_login() -> tokio_postgres::Client {
    let db = db_config();
    connect(&db.app_user, &db.app_password).await
}

async fn connect(user: &str, password: &str) -> tokio_postgres::Client {
    let db = db_config();
    let (client, conn) = crm::db::pg_config(&db, user, password)
        .connect(tokio_postgres::NoTls)
        .await
        .unwrap();
    tokio::spawn(conn);
    client
}

pub struct TestApp {
    pub base: String,
}

impl TestApp {
    pub async fn start() -> TestApp {
        Self::start_with(|_| {}).await
    }

    /// An app whose attachments bucket is a real RustFS.
    pub async fn start_with_files() -> TestApp {
        let endpoint = std::thread::spawn(|| rustfs().endpoint.clone())
            .join()
            .unwrap();
        Self::start_with(move |c| c.storage.endpoint = endpoint).await
    }

    pub async fn start_with(customize: impl FnOnce(&mut Config)) -> TestApp {
        // Server errors in the output of a failed test.
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_env_filter("warn,crm=info")
            .try_init();
        let mut config = Config {
            port: 0,
            db: db_config(),
            auth: AuthConfig {
                max_failed_logins: MAX_FAILED,
                lock_minutes: 15,
                admin_login: Some(ADMIN_LOGIN.into()),
                admin_password: Some(ADMIN_PASSWORD.into()),
                dev_login: false,
                session_timeout: Duration::from_secs(1800),
                cookie_secure: false,
            },
            static_dir: "does-not-exist".into(),
            swagger_ui: true,
            desk_public_reports_per_minute: 1000,
            // Not started unless a test asks for it (start_with_files): nothing listens on port 9.
            storage: crm::storage::StorageConfig {
                endpoint: "http://127.0.0.1:9".into(),
                region: "us-east-1".into(),
                access_key: S3_ACCESS_KEY.into(),
                secret_key: S3_SECRET_KEY.into(),
                bucket: S3_BUCKET.into(),
            },
        };
        customize(&mut config);
        let config = Arc::new(config);
        let state = AppState::new(crm::db::app_pool(&config.db).unwrap(), config);
        crm::auth::ensure_admin(&state).await.unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let app = crm::router(state, MemorySessions::default());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        TestApp { base }
    }

    pub fn client(&self) -> Api {
        let jar = Arc::new(reqwest::cookie::Jar::default());
        Api {
            base: self.base.clone(),
            jar: jar.clone(),
            http: reqwest::Client::builder()
                .cookie_provider(jar)
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap(),
        }
    }

    pub async fn logged_in(&self, login: &str, password: &str) -> Api {
        let c = self.client();
        let r = c.login(login, password).await;
        assert_eq!(r.status, 200, "login {login}: {:?}", r.body);
        c
    }

    pub async fn admin(&self) -> Api {
        self.logged_in(ADMIN_LOGIN, ADMIN_PASSWORD).await
    }

    /// A new user created by the administrator; returns (id, login).
    pub async fn new_user(&self, role: &str, password: &str) -> (i64, String) {
        let login = format!("t.{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);
        let r = self
            .admin()
            .await
            .post(
                "/api/users",
                serde_json::json!({"login": login, "displayName": format!("Test {login}"),
                    "role": role, "password": password}),
            )
            .await;
        assert_eq!(r.status, 201, "{:?}", r.body);
        (r.body["id"].as_i64().unwrap(), login)
    }
}

pub struct Response {
    pub status: u16,
    pub body: Value,
    pub headers: reqwest::header::HeaderMap,
}

/// A real HTTP client with its own cookie jar (session + XSRF-TOKEN), like a browser.
pub struct Api {
    base: String,
    jar: Arc<reqwest::cookie::Jar>,
    http: reqwest::Client,
}

impl Api {
    pub fn cookie(&self, name: &str) -> Option<String> {
        let url = self.base.parse().unwrap();
        let header = self.jar.cookies(&url)?;
        header
            .to_str()
            .ok()?
            .split(';')
            .filter_map(|p| p.trim().split_once('='))
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v.to_owned())
    }

    pub async fn csrf(&self) -> Response {
        self.send(Method::GET, "/api/auth/csrf", None, false).await
    }

    pub async fn login(&self, login: &str, password: &str) -> Response {
        self.csrf().await;
        self.post(
            "/api/auth/login",
            serde_json::json!({"login": login, "password": password}),
        )
        .await
    }

    pub async fn get(&self, path: &str) -> Response {
        self.send(Method::GET, path, None, true).await
    }

    pub async fn post(&self, path: &str, body: Value) -> Response {
        self.send(Method::POST, path, Some(body), true).await
    }

    pub async fn post_without_csrf(&self, path: &str, body: Value) -> Response {
        self.send(Method::POST, path, Some(body), false).await
    }

    pub async fn put(&self, path: &str, body: Value) -> Response {
        self.send(Method::PUT, path, Some(body), true).await
    }

    pub async fn delete(&self, path: &str) -> Response {
        self.send(Method::DELETE, path, None, true).await
    }

    pub async fn send(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
        csrf: bool,
    ) -> Response {
        let mut request = self.http.request(method, format!("{}{}", self.base, path));
        if let Some(body) = body {
            request = request.json(&body);
        }
        if csrf && let Some(token) = self.cookie("XSRF-TOKEN") {
            request = request.header("X-XSRF-TOKEN", token);
        }
        let response = request.send().await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let text = response.text().await.unwrap();
        Response {
            status: status.as_u16(),
            body: serde_json::from_str(&text).unwrap_or(Value::Null),
            headers,
        }
    }
}

pub const PASSWORD: &str = "Start-Passw0rd";

/// A team built through the API: a manager leading it and `sales` SALES members.
pub struct Team {
    pub id: i64,
    pub manager: (i64, String),
    pub sales: Vec<(i64, String)>,
}

impl TestApp {
    pub async fn team(&self, sales: usize) -> Team {
        let manager = self.new_user("MANAGER", PASSWORD).await;
        let admin = self.admin().await;
        let name = format!("Test team {}", uuid::Uuid::new_v4().simple());
        let r = admin
            .post(
                "/api/teams",
                serde_json::json!({"name": name, "leadUserId": manager.0}),
            )
            .await;
        assert_eq!(r.status, 201, "{:?}", r.body);
        let id = r.body["id"].as_i64().unwrap();
        let mut members = Vec::new();
        for _ in 0..sales {
            let user = self.new_user("SALES", PASSWORD).await;
            let r = admin
                .post(
                    &format!("/api/teams/{id}/members"),
                    serde_json::json!({"userId": user.0}),
                )
                .await;
            assert_eq!(r.status, 204, "{:?}", r.body);
            members.push(user);
        }
        Team {
            id,
            manager,
            sales: members,
        }
    }

    pub async fn as_user(&self, login: &str) -> Api {
        self.logged_in(login, PASSWORD).await
    }
}

/// A new client of the logged-in user (SALES: assigned to them).
pub async fn new_client(api: &Api, name: &str) -> Value {
    let r = api
        .post(
            "/api/clients",
            serde_json::json!({"name": name, "status": "ACTIVE"}),
        )
        .await;
    assert_eq!(r.status, 201, "{:?}", r.body);
    r.body
}

/// A NEW project of `client_id` in `team_id`, owned by `owner_id`, due on `due`.
pub async fn new_project(
    api: &Api,
    client_id: i64,
    team_id: i64,
    owner_id: i64,
    due: chrono::NaiveDate,
) -> Value {
    let r = api
        .post(
            "/api/projects",
            serde_json::json!({
                "clientId": client_id, "teamId": team_id, "ownerUserId": owner_id, "name": "Test projekt",
                "startDate": (due - chrono::Days::new(30)).to_string(), "dueDate": due.to_string(), "budget": 1234.56
            }),
        )
        .await;
    assert_eq!(r.status, 201, "{:?}", r.body);
    r.body
}

pub fn today() -> chrono::NaiveDate {
    chrono::Utc::now().date_naive()
}

pub fn ids(page: &Value) -> Vec<i64> {
    page["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_i64().unwrap())
        .collect()
}

/// A person (requester or handler) of the desk. People come from the seed in the real system; tests add them
/// directly. Returns (id, "First Last").
pub async fn person(department: &str) -> (i32, String) {
    let tag = &uuid::Uuid::new_v4().simple().to_string()[..8];
    let (first, last) = (format!("Imię{tag}"), format!("Nazwisko{tag}"));
    let row = superuser()
        .await
        .query_one(
            "INSERT INTO crm.people (first_name, last_name, department, email) VALUES ($1, $2, $3, $4) RETURNING id",
            &[&first, &last, &department, &format!("{tag}@company.test")],
        )
        .await
        .unwrap();
    (row.get(0), format!("{first} {last}"))
}

/// A ticket registered by `api` for a new requester.
pub async fn report_ticket(
    api: &Api,
    priority: &str,
    reported_at: chrono::DateTime<chrono::Utc>,
    title: &str,
) -> Value {
    let (requester, _) = person("Sprzedaż").await;
    let r = api
        .post(
            "/api/desk/tickets",
            serde_json::json!({
                "title": title, "description": "Nie włącza się po aktualizacji.", "categoryKey": "it",
                "subcategoryKey": "it.hardware", "priority": priority, "reportedAt": reported_at.to_rfc3339(),
                "requesterPersonId": requester
            }),
        )
        .await;
    assert_eq!(r.status, 201, "{:?}", r.body);
    r.body
}

impl Api {
    /// GET returning the raw bytes and headers (downloads).
    pub async fn download(&self, path: &str) -> (u16, reqwest::header::HeaderMap, Vec<u8>) {
        let response = self
            .http
            .get(format!("{}{}", self.base, path))
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        (status, headers, response.bytes().await.unwrap().to_vec())
    }

    /// multipart/form-data POST with the CSRF header, like a browser's FormData.
    pub async fn multipart(&self, path: &str, form: reqwest::multipart::Form) -> Response {
        let mut request = self
            .http
            .post(format!("{}{}", self.base, path))
            .multipart(form);
        if let Some(token) = self.cookie("XSRF-TOKEN") {
            request = request.header("X-XSRF-TOKEN", token);
        }
        let response = request.send().await.unwrap();
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let text = response.text().await.unwrap();
        Response {
            status,
            body: serde_json::from_str(&text).unwrap_or(Value::Null),
            headers,
        }
    }

    pub async fn me(&self) -> i64 {
        self.get("/api/auth/me").await.body["id"].as_i64().unwrap()
    }
}

//! The seed (ADR-0004) is the same procedure as for the performance data set, run here with small numbers.

mod support;

use crm::seed::SeedConfig;
use support::*;
use tokio::sync::OnceCell;

const SEED_PASSWORD: &str = "Seed-Passw0rd";

fn small() -> SeedConfig {
    SeedConfig {
        clients: 2_500,
        admins: 1,
        managers: 2,
        sales: 6,
        team_size: 5,
        projects: 2_100,
        comments: 1_100,
        random_seed: 42,
        // Three statements for the clients, so the batches meet.
        batch_size: 1_000,
        password: SEED_PASSWORD.into(),
        desk_tickets: 20,
        desk_only: false,
    }
}

static SEEDED: OnceCell<()> = OnceCell::const_new();

/// The test database, seeded once for every test of this file.
async fn seeded() {
    SEEDED
        .get_or_init(|| async {
            crm::seed::run(&db_config(), &small()).await.expect("seed");
        })
        .await;
}

async fn count(sql: &str) -> i64 {
    superuser().await.query_one(sql, &[]).await.unwrap().get(0)
}

#[tokio::test]
async fn seed_creates_the_requested_rows_consistent_with_each_other() {
    seeded().await;
    assert_eq!(
        count("SELECT count(*) FROM crm.users WHERE login LIKE 'perf.%'").await,
        9
    );
    assert_eq!(
        count("SELECT count(*) FROM crm.teams WHERE name LIKE 'Perf Team %'").await,
        2
    );
    assert_eq!(count("SELECT count(*) FROM crm.team_members").await, 6);
    assert_eq!(
        count("SELECT count(*) FROM crm.clients WHERE email LIKE 'klient%@example.test'").await,
        2_500
    );
    assert_eq!(
        count("SELECT count(*) FROM crm.projects WHERE created_by IS NULL").await,
        2_100
    );
    assert_eq!(
        count("SELECT count(*) FROM crm.project_stage_history WHERE changed_by IS NULL").await,
        2_100
    );
    assert_eq!(
        count("SELECT count(*) FROM crm.project_comments").await,
        1_100
    );
    // Every seeded project is owned by the client's sales user, a member of the project's team.
    assert_eq!(
        count(
            "SELECT count(*) FROM crm.projects AS p JOIN crm.clients AS c ON c.id = p.client_id
             WHERE c.assigned_user_id <> p.owner_user_id
                OR NOT EXISTS (SELECT 1 FROM crm.team_members AS tm
                               WHERE tm.team_id = p.team_id AND tm.user_id = p.owner_user_id)"
        )
        .await,
        0
    );
    // Comments are written by the project owner.
    assert_eq!(
        count(
            "SELECT count(*) FROM crm.project_comments AS m JOIN crm.projects AS p ON p.id = m.project_id
             WHERE m.author_id <> p.owner_user_id"
        )
        .await,
        0
    );
    // The team statistics the procedures keep in step agree with the projects.
    assert_eq!(
        count(
            "SELECT count(*) FROM (
                 SELECT team_id, stage, count(*) AS n, sum(budget) AS b FROM crm.projects GROUP BY 1, 2
                 EXCEPT
                 SELECT team_id, stage, project_count, budget_total FROM crm.team_project_stats) AS diff"
        )
        .await,
        0
    );
    // Identity sequences continue after the seeded ids.
    assert_eq!(
        count("SELECT last_value - (SELECT max(id) FROM crm.clients) FROM crm.clients_id_seq")
            .await,
        0
    );
    // Indexes dropped for the load are back, the statistics are fresh.
    for index in [
        "ix_clients_name",
        "ix_clients_assigned_user_name",
        "ux_clients_nip",
        "ix_projects_open_due",
        "ux_project_stage_history_project",
        "ux_project_comments_project",
    ] {
        assert_eq!(
            count(&format!(
                "SELECT count(*) FROM pg_indexes WHERE schemaname = 'crm' AND indexname = '{index}'"
            ))
            .await,
            1,
            "{index}"
        );
    }
    assert!(
        count("SELECT count(*) FROM pg_stats WHERE schemaname = 'crm' AND tablename = 'clients'")
            .await
            > 0,
        "ANALYZE ran"
    );
    // The desk: ten agents, the people, tickets with real streams.
    assert_eq!(count("SELECT count(*) FROM crm.tickets").await, 20);
    assert_eq!(
        count("SELECT count(*) FROM crm.users WHERE login = 'anna.nowak'").await,
        1
    );
    assert!(
        count("SELECT count(*) FROM crm.ticket_events WHERE event_type = 'TicketTaken'").await > 0
    );
}

#[tokio::test]
async fn hash_numbers_are_those_of_the_t_sql_seed() {
    seeded().await;
    let db = superuser().await;
    // CAST(0xFFFFFFFF AS int) & 0x7FFFFFFF and CAST(0x00000102 AS int) & 0x7FFFFFFF in T-SQL.
    let row = db
        .query_one(
            "SELECT seed.h('\\xffffffff00000102'::bytea, 0), seed.h('\\xffffffff00000102'::bytea, 1),
                    encode(seed.hash(42, 'c', 1), 'hex') = encode(sha256('42:c:1'), 'hex')",
            &[],
        )
        .await
        .unwrap();
    assert_eq!(row.get::<_, i64>(0), 2_147_483_647);
    assert_eq!(row.get::<_, i64>(1), 258);
    assert!(row.get::<_, bool>(2));
}

#[tokio::test]
async fn the_same_parameters_give_the_same_data() {
    seeded().await;
    // A second database, migrated and seeded with the same parameters.
    let mut other = db_config();
    other.name = "crm_seed_twin".into();
    crm::migrate::init(&other).await.unwrap();
    crm::migrate::migrate(&other).await.unwrap();
    crm::seed::run(&other, &small()).await.unwrap();

    let fingerprint = |db: crm::config::DbConfig| async move {
        let (client, conn) = crm::db::pg_config(&db, &db.admin_user, &db.admin_password)
            .connect(tokio_postgres::NoTls)
            .await
            .unwrap();
        tokio::spawn(conn);
        let mut sums = Vec::new();
        for sql in [
            "SELECT md5(string_agg(concat_ws('|', c.name, c.nip, c.email, c.phone, c.city, c.status, u.login, c.note,
                                             c.created_at - (SELECT min(created_at) FROM crm.clients)),
                                   ',' ORDER BY c.nip))
             FROM crm.clients AS c JOIN crm.users AS u ON u.id = c.assigned_user_id",
            "SELECT md5(string_agg(concat_ws('|', p.name, c.nip, t.name, u.login, p.stage, p.start_date, p.due_date,
                                             p.budget), ',' ORDER BY p.name))
             FROM crm.projects AS p JOIN crm.clients AS c ON c.id = p.client_id
             JOIN crm.teams AS t ON t.id = p.team_id JOIN crm.users AS u ON u.id = p.owner_user_id",
            "SELECT md5(string_agg(concat_ws('|', p.name, u.login, m.body,
                                             m.created_at - (SELECT min(created_at) FROM crm.project_comments)),
                                   ',' ORDER BY p.name, m.body, m.created_at))
             FROM crm.project_comments AS m JOIN crm.projects AS p ON p.id = m.project_id
             JOIN crm.users AS u ON u.id = m.author_id",
            "SELECT md5(string_agg(concat_ws('|', u.login, u.role, t.name, l.login), ',' ORDER BY u.login))
             FROM crm.team_members AS m JOIN crm.users AS u ON u.id = m.user_id JOIN crm.teams AS t ON t.id = m.team_id
             JOIN crm.users AS l ON l.id = t.lead_user_id WHERE u.login LIKE 'perf.%'",
            "SELECT md5(string_agg(concat_ws('|', t.title, t.category_key, t.priority, t.status, t.channel,
                                             t.requester_incognito), ',' ORDER BY t.title)) FROM crm.tickets AS t",
        ] {
            sums.push(client.query_one(sql, &[]).await.unwrap().get::<_, String>(0));
        }
        sums
    };
    // Compared by logins, NIPs and names: the ids depend on how many users existed before the seed.
    assert_eq!(fingerprint(db_config()).await, fingerprint(other).await);
}

#[tokio::test]
async fn the_seed_refuses_to_run_twice_and_rejects_invalid_parameters() {
    seeded().await;
    let error = crm::seed::run(&db_config(), &small()).await.unwrap_err();
    let code = error
        .chain()
        .find_map(|e| e.downcast_ref::<tokio_postgres::Error>())
        .and_then(|e| e.code())
        .map(|c| c.code().to_owned());
    assert_eq!(code.as_deref(), Some("U1060"), "{error:#}");

    let db = superuser().await;
    let invalid = db
        .execute("CALL seed.run(-1, 0, 1, 1, 'x', 5, 0, 0, 42, 1000)", &[])
        .await
        .unwrap_err();
    assert_eq!(invalid.code().unwrap().code(), "U1050");
    let batch = db
        .execute("CALL seed.run(1, 0, 1, 1, 'x', 5, 0, 0, 42, 10)", &[])
        .await
        .unwrap_err();
    assert_eq!(batch.code().unwrap().code(), "U1050");
    let no_hash = db
        .execute("CALL seed.desk_run('', 10, 42)", &[])
        .await
        .unwrap_err();
    assert_eq!(no_hash.code().unwrap().code(), "U1020");
}

#[tokio::test]
async fn the_desk_part_alone_is_idempotent() {
    seeded().await;
    let before = (
        count("SELECT count(*) FROM crm.users").await,
        count("SELECT count(*) FROM crm.people").await,
        count("SELECT count(*) FROM crm.tickets").await,
    );
    let desk_only = SeedConfig {
        desk_only: true,
        ..small()
    };
    crm::seed::run(&db_config(), &desk_only).await.unwrap();
    let after = (
        count("SELECT count(*) FROM crm.users").await,
        count("SELECT count(*) FROM crm.people").await,
        count("SELECT count(*) FROM crm.tickets").await,
    );
    assert_eq!(before, after);
}

#[tokio::test]
async fn seeded_users_sign_in_through_the_api() {
    seeded().await;
    let app = TestApp::start().await;
    let sales = app.logged_in("perf.sales.0001", SEED_PASSWORD).await;
    let me = sales.get("/api/auth/me").await.body;
    assert_eq!(me["role"], "SALES");
    let clients = sales.get("/api/clients?size=5").await;
    assert_eq!(clients.status, 200);
    assert!(
        !clients.body["items"].as_array().unwrap().is_empty(),
        "a sales user sees own clients"
    );
    let agent = app.logged_in("anna.nowak", SEED_PASSWORD).await;
    assert_eq!(agent.get("/api/desk/tickets/counters").await.status, 200);
}

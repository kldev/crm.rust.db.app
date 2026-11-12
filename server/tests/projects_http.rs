//! Projects, stages, comments and team statistics over HTTP (ProjectHttpTest.kt).

mod support;

use chrono::Days;
use serde_json::{Value, json};
use support::*;

async fn change_stage(api: &Api, project: &Value, stage: &str, comment: Option<&str>) -> Response {
    api.post(
        &format!("/api/projects/{}/stage", project["id"]),
        json!({"stage": stage, "comment": comment, "rowVersion": project["rowVersion"]}),
    )
    .await
}

/// A sales member of a new team, their client and a NEW project due in 60 days.
async fn setup() -> (TestApp, Team, Api, Value) {
    let app = TestApp::start().await;
    let team = app.team(2).await;
    let c = app.as_user(&team.sales[0].1).await;
    let client = new_client(&c, "Test Projektowy").await;
    let project = new_project(
        &c,
        client["id"].as_i64().unwrap(),
        team.id,
        team.sales[0].0,
        today() + Days::new(60),
    )
    .await;
    (app, team, c, project)
}

#[tokio::test]
async fn new_project_starts_in_new_with_one_history_row() {
    let (_app, _team, c, project) = setup().await;
    assert_eq!(project["stage"], "NEW");
    assert_eq!(project["overdue"], false);
    assert_eq!(
        project["budget"].to_string(),
        "1234.56",
        "budget is an exact JSON number"
    );
    let history = c
        .get(&format!("/api/projects/{}/stages", project["id"]))
        .await
        .body;
    assert_eq!(history.as_array().unwrap().len(), 1);
    assert!(history[0]["fromStage"].is_null());
}

#[tokio::test]
async fn allowed_stage_change_writes_history_and_the_optional_comment_in_one_go() {
    let (_app, _team, c, project) = setup().await;
    let accepted = change_stage(&c, &project, "ACCEPTED", Some("Test: offer accepted")).await;
    assert_eq!(accepted.status, 200, "{:?}", accepted.body);
    assert_eq!(accepted.body["stage"], "ACCEPTED");
    let history = c
        .get(&format!("/api/projects/{}/stages", project["id"]))
        .await
        .body;
    let stages: Vec<_> = history
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["toStage"].as_str().unwrap())
        .collect();
    assert_eq!(stages, ["ACCEPTED", "NEW"]);
    let comments = c
        .get(&format!("/api/projects/{}/comments", project["id"]))
        .await
        .body;
    assert_eq!(comments["items"][0]["body"], "Test: offer accepted");
}

#[tokio::test]
async fn stage_change_outside_the_transition_table_is_409() {
    let (_app, _team, c, project) = setup().await;
    let r = change_stage(&c, &project, "CLOSED", None).await;
    assert_eq!(r.status, 409);
    assert_eq!(r.body["errorNumber"], 51902);
    assert_eq!(change_stage(&c, &project, "DONE", None).await.status, 400);
}

#[tokio::test]
async fn same_stage_is_a_no_op_and_stale_row_version_is_409() {
    let (_app, _team, c, project) = setup().await;
    let same = change_stage(&c, &project, "NEW", Some("Test: ignored")).await;
    assert_eq!(same.status, 200);
    assert_eq!(same.body["rowVersion"], project["rowVersion"]);
    let comments = c
        .get(&format!("/api/projects/{}/comments", project["id"]))
        .await
        .body;
    assert!(comments["items"].as_array().unwrap().is_empty());

    assert_eq!(
        change_stage(&c, &project, "ACCEPTED", None).await.status,
        200
    );
    let stale = change_stage(&c, &project, "DECLINED", None).await;
    assert_eq!(stale.status, 409);
    assert_eq!(stale.body["errorNumber"], 51901);
}

#[tokio::test]
async fn two_stage_changes_with_the_same_row_version_one_wins() {
    let (app, team, c, project) = setup().await;
    let mate = app.as_user(&team.sales[1].1).await;
    let (a, b) = tokio::join!(
        change_stage(&c, &project, "ACCEPTED", None),
        change_stage(&mate, &project, "DECLINED", None)
    );
    let mut statuses = [a.status, b.status];
    statuses.sort();
    assert_eq!(statuses, [200, 409]);
    let loser = if a.status == 409 { a } else { b };
    assert_eq!(loser.body["errorNumber"], 51901);
}

#[tokio::test]
async fn closed_project_cannot_be_edited() {
    let (_app, _team, c, mut p) = setup().await;
    for stage in ["ACCEPTED", "ACTIVE", "CLOSED"] {
        p = change_stage(&c, &p, stage, None).await.body;
    }
    let r = c
        .put(
            &format!("/api/projects/{}", p["id"]),
            json!({"teamId": p["teamId"], "ownerUserId": p["ownerUserId"], "name": "Test changed",
                   "startDate": p["startDate"], "dueDate": p["dueDate"], "budget": 1, "rowVersion": p["rowVersion"]}),
        )
        .await;
    assert_eq!(r.status, 409);
    assert_eq!(r.body["errorNumber"], 51902);
}

#[tokio::test]
async fn update_validates_the_request_and_identical_values_keep_the_row_version() {
    let (_app, _team, c, p) = setup().await;
    let body = |budget: Value| {
        json!({"teamId": p["teamId"], "ownerUserId": p["ownerUserId"], "name": p["name"],
               "startDate": p["startDate"], "dueDate": p["dueDate"], "budget": budget, "rowVersion": p["rowVersion"]})
    };
    let same = c
        .put(&format!("/api/projects/{}", p["id"]), body(json!(1234.56)))
        .await;
    assert_eq!(same.status, 200, "{:?}", same.body);
    assert_eq!(same.body["rowVersion"], p["rowVersion"]);
    assert_eq!(
        c.put(&format!("/api/projects/{}", p["id"]), body(json!(1.234)))
            .await
            .status,
        400
    );
    assert_eq!(
        c.put(&format!("/api/projects/{}", p["id"]), body(json!(-1)))
            .await
            .status,
        400
    );
    let dates = c
        .put(
            &format!("/api/projects/{}", p["id"]),
            json!({"teamId": p["teamId"], "ownerUserId": p["ownerUserId"], "name": "X", "startDate": "2026-05-01",
                   "dueDate": "2026-04-01", "budget": 1, "rowVersion": p["rowVersion"]}),
        )
        .await;
    assert_eq!(dates.body["errorNumber"], 51032);
}

#[tokio::test]
async fn owner_must_be_a_member_of_the_project_team() {
    let app = TestApp::start().await;
    let team = app.team(1).await;
    let other = app.team(1).await;
    let manager = app.as_user(&team.manager.1).await;
    let client = new_client(&manager, "Test owner rule").await;
    let r = manager
        .post(
            "/api/projects",
            json!({"clientId": client["id"], "teamId": other.id, "ownerUserId": team.sales[0].0, "name": "Test",
                   "startDate": "2026-01-01", "dueDate": "2026-02-01", "budget": 10}),
        )
        .await;
    assert_eq!(r.status, 400);
    assert_eq!(r.body["errorNumber"], 51037);
}

#[tokio::test]
async fn sales_user_sees_own_team_and_own_client_projects_only() {
    let app = TestApp::start().await;
    let team = app.team(2).await;
    let other = app.team(1).await;
    let me = app.as_user(&team.sales[0].1).await;
    let foreigner = app.as_user(&other.sales[0].1).await;

    // A project of another team for another client: invisible.
    let foreign_client = new_client(&foreigner, "Test foreign").await;
    let foreign = new_project(
        &foreigner,
        foreign_client["id"].as_i64().unwrap(),
        other.id,
        other.sales[0].0,
        today(),
    )
    .await;
    assert_eq!(
        me.get(&format!("/api/projects/{}", foreign["id"]))
            .await
            .status,
        404
    );

    // A project of my team (owned by a team mate): visible.
    let mate = app.as_user(&team.sales[1].1).await;
    let mate_client = new_client(&mate, "Test mate").await;
    let of_team = new_project(
        &mate,
        mate_client["id"].as_i64().unwrap(),
        team.id,
        team.sales[1].0,
        today(),
    )
    .await;
    assert_eq!(
        me.get(&format!("/api/projects/{}", of_team["id"]))
            .await
            .status,
        200
    );

    // A project of another team for my client (created by a manager): visible.
    let my_client = new_client(&me, "Test my client").await;
    let other_manager = app.as_user(&other.manager.1).await;
    let for_my_client = new_project(
        &other_manager,
        my_client["id"].as_i64().unwrap(),
        other.id,
        other.sales[0].0,
        today(),
    )
    .await;
    assert_eq!(
        me.get(&format!("/api/projects/{}", for_my_client["id"]))
            .await
            .status,
        200
    );

    let visible = ids(&me.get("/api/projects?size=200").await.body);
    assert!(!visible.contains(&foreign["id"].as_i64().unwrap()));
    assert!(visible.contains(&of_team["id"].as_i64().unwrap()));
    assert!(visible.contains(&for_my_client["id"].as_i64().unwrap()));
}

#[tokio::test]
async fn overdue_filter_returns_only_open_projects_due_before_today_paged_by_due_date() {
    let app = TestApp::start().await;
    let team = app.team(1).await;
    let manager = app.as_user(&team.manager.1).await;
    let client = new_client(&manager, "Test overdue").await;
    let client_id = client["id"].as_i64().unwrap();
    let owner = team.sales[0].0;
    for days in [1u64, 3, 3, 10, 40] {
        new_project(
            &manager,
            client_id,
            team.id,
            owner,
            today() - Days::new(days),
        )
        .await;
    }
    new_project(&manager, client_id, team.id, owner, today()).await; // due today: not overdue
    let closed = new_project(&manager, client_id, team.id, owner, today() - Days::new(5)).await;
    let declined = change_stage(&manager, &closed, "DECLINED", None).await;
    assert_eq!(declined.status, 200);

    let mut all = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let url = format!(
            "/api/projects?overdue=true&clientId={client_id}&size=2{}",
            cursor
                .as_ref()
                .map(|c| format!("&cursor={c}"))
                .unwrap_or_default()
        );
        let page = manager.get(&url).await.body;
        all.extend(page["items"].as_array().unwrap().clone());
        cursor = page["nextCursor"].as_str().map(str::to_owned);
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(all.len(), 5);
    assert!(all.iter().all(|p| p["overdue"] == true));
    let dues: Vec<_> = all
        .iter()
        .map(|p| p["dueDate"].as_str().unwrap().to_owned())
        .collect();
    let mut sorted = dues.clone();
    sorted.sort();
    assert_eq!(dues, sorted);
    assert_eq!(
        manager
            .get(&format!(
                "/api/projects/count?overdue=true&clientId={client_id}"
            ))
            .await
            .body["total"],
        5
    );
}

#[tokio::test]
async fn count_matches_the_list_for_every_page_of_a_sales_user() {
    let app = TestApp::start().await;
    let team = app.team(2).await;
    let me = app.as_user(&team.sales[0].1).await;
    let mate = app.as_user(&team.sales[1].1).await;
    let mine = new_client(&me, "Test count mine").await;
    let theirs = new_client(&mate, "Test count mate").await;
    for days in 0..4u64 {
        new_project(
            &me,
            mine["id"].as_i64().unwrap(),
            team.id,
            team.sales[0].0,
            today() + Days::new(days),
        )
        .await;
        new_project(
            &mate,
            theirs["id"].as_i64().unwrap(),
            team.id,
            team.sales[1].0,
            today() + Days::new(days),
        )
        .await;
    }
    let mut shown = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let url = format!(
            "/api/projects?size=3{}",
            cursor
                .as_ref()
                .map(|c| format!("&cursor={c}"))
                .unwrap_or_default()
        );
        let page = me.get(&url).await.body;
        shown.extend(ids(&page));
        cursor = page["nextCursor"].as_str().map(str::to_owned);
        if cursor.is_none() {
            break;
        }
    }
    let unique: std::collections::HashSet<_> = shown.iter().collect();
    assert_eq!(unique.len(), shown.len(), "pages do not overlap");
    assert_eq!(shown.len(), 8);
    assert_eq!(me.get("/api/projects/count").await.body["total"], 8);
    assert_eq!(me.get("/api/projects/count?stage=NOPE").await.status, 400);
    assert_eq!(me.get("/api/projects?dueFrom=yesterday").await.status, 400);
}

#[tokio::test]
async fn team_statistics_follow_every_write() {
    let app = TestApp::start().await;
    let team = app.team(1).await;
    let other = app.team(1).await;
    let manager = app.as_user(&team.manager.1).await;
    let admin = app.admin().await;
    admin
        .post(
            &format!("/api/teams/{}/members", other.id),
            json!({"userId": team.sales[0].0}),
        )
        .await;
    let client = new_client(&manager, "Test stats").await;
    let p = new_project(
        &manager,
        client["id"].as_i64().unwrap(),
        team.id,
        team.sales[0].0,
        today(),
    )
    .await;
    let p = change_stage(&manager, &p, "ACCEPTED", None).await.body;
    // Move the project to the other team with a new budget.
    let moved = manager
        .put(
            &format!("/api/projects/{}", p["id"]),
            json!({"teamId": other.id, "ownerUserId": team.sales[0].0, "name": "Test moved", "startDate": p["startDate"],
                   "dueDate": p["dueDate"], "budget": 100.5, "rowVersion": p["rowVersion"]}),
        )
        .await;
    assert_eq!(moved.status, 200, "{:?}", moved.body);

    let db = superuser().await;
    for team_id in [team.id, other.id] {
        let stats = manager
            .get(&format!("/api/teams/{team_id}/stats"))
            .await
            .body;
        let rows = db
            .query(
                "SELECT stage, count(*), sum(budget)::text FROM crm.projects WHERE team_id = $1 GROUP BY stage ORDER BY stage",
                &[&(team_id as i32)],
            )
            .await
            .unwrap();
        let expected: Vec<(String, i64, String)> = rows
            .iter()
            .map(|r| (r.get(0), r.get(1), r.get(2)))
            .collect();
        let actual: Vec<(String, i64, String)> = stats
            .as_array()
            .unwrap()
            .iter()
            .map(|s| {
                (
                    s["stage"].as_str().unwrap().to_owned(),
                    s["projectCount"].as_i64().unwrap(),
                    s["budgetTotal"].to_string(),
                )
            })
            .collect();
        assert_eq!(actual, expected, "team {team_id}");
    }
    assert_eq!(manager.get("/api/teams/999999/stats").await.status, 404);
}

#[tokio::test]
async fn comments_are_paged_newest_first() {
    let (_app, _team, c, project) = setup().await;
    let id = project["id"].clone();
    for i in 0..3 {
        let r = c
            .post(
                &format!("/api/projects/{id}/comments"),
                json!({"body": format!("Test {i}")}),
            )
            .await;
        assert_eq!(r.status, 201);
        assert_eq!(r.body["body"], format!("Test {i}"));
    }
    let first = c
        .get(&format!("/api/projects/{id}/comments?size=2"))
        .await
        .body;
    let bodies: Vec<_> = first["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["body"].as_str().unwrap())
        .collect();
    assert_eq!(bodies, ["Test 2", "Test 1"]);
    let second = c
        .get(&format!(
            "/api/projects/{id}/comments?size=2&cursor={}",
            first["nextCursor"].as_str().unwrap()
        ))
        .await
        .body;
    assert_eq!(second["items"][0]["body"], "Test 0");
    assert_eq!(second["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        c.post(
            &format!("/api/projects/{id}/comments"),
            json!({"body": "   "})
        )
        .await
        .status,
        400
    );
}

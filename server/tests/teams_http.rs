//! Teams and members over HTTP.

mod support;

use serde_json::json;
use support::*;

#[tokio::test]
async fn only_admin_creates_teams_led_by_an_active_manager_with_a_unique_name() {
    let app = TestApp::start().await;
    let (manager_id, manager) = app.new_user("MANAGER", PASSWORD).await;
    let (sales_id, _) = app.new_user("SALES", PASSWORD).await;
    let m = app.as_user(&manager).await;
    let name = format!("Team {}", uuid::Uuid::new_v4().simple());
    assert_eq!(
        m.post(
            "/api/teams",
            json!({"name": name, "leadUserId": manager_id})
        )
        .await
        .status,
        403
    );

    let admin = app.admin().await;
    let not_manager = admin
        .post("/api/teams", json!({"name": name, "leadUserId": sales_id}))
        .await;
    assert_eq!(not_manager.body["errorNumber"], 51035);
    let created = admin
        .post(
            "/api/teams",
            json!({"name": name, "leadUserId": manager_id}),
        )
        .await;
    assert_eq!(created.status, 201, "{:?}", created.body);
    assert_eq!(created.body["leadLogin"], manager.as_str());
    assert_eq!(created.body["memberCount"], 0);
    let dup = admin
        .post(
            "/api/teams",
            json!({"name": name.to_uppercase(), "leadUserId": manager_id}),
        )
        .await;
    assert_eq!(dup.status, 409, "names are unique regardless of case");
    assert_eq!(
        admin
            .post("/api/teams", json!({"leadUserId": manager_id}))
            .await
            .status,
        400
    );
}

#[tokio::test]
async fn the_lead_manages_members_and_an_owner_of_open_projects_stays() {
    let app = TestApp::start().await;
    let team = app.team(1).await;
    let (stranger_id, stranger) = app.new_user("MANAGER", PASSWORD).await;
    let lead = app.as_user(&team.manager.1).await;
    let (new_member, _) = app.new_user("SALES", PASSWORD).await;

    let r = lead
        .post(
            &format!("/api/teams/{}/members", team.id),
            json!({"userId": new_member}),
        )
        .await;
    assert_eq!(r.status, 204);
    // Adding again changes nothing.
    assert_eq!(
        lead.post(
            &format!("/api/teams/{}/members", team.id),
            json!({"userId": new_member})
        )
        .await
        .status,
        204
    );
    let members = lead
        .get(&format!("/api/teams/{}/members", team.id))
        .await
        .body;
    assert_eq!(members.as_array().unwrap().len(), 2);
    assert!(members[0]["joinedAt"].is_string());

    let other_manager = app.as_user(&stranger).await;
    let denied = other_manager
        .post(
            &format!("/api/teams/{}/members", team.id),
            json!({"userId": stranger_id}),
        )
        .await;
    assert_eq!(denied.status, 403);
    assert_eq!(denied.body["errorNumber"], 51301);

    let owner = &team.sales[0];
    let client = new_client(&lead, "Test members").await;
    new_project(
        &lead,
        client["id"].as_i64().unwrap(),
        team.id,
        owner.0,
        today(),
    )
    .await;
    let blocked = lead
        .delete(&format!("/api/teams/{}/members/{}", team.id, owner.0))
        .await;
    assert_eq!(blocked.status, 409);
    assert_eq!(blocked.body["errorNumber"], 51905);
    assert_eq!(
        lead.delete(&format!("/api/teams/{}/members/{new_member}", team.id))
            .await
            .status,
        204
    );
    assert_eq!(lead.get("/api/teams/999999/members").await.status, 404);
}

#[tokio::test]
async fn teams_are_paged_by_id() {
    let app = TestApp::start().await;
    for _ in 0..3 {
        app.team(0).await;
    }
    let (_, sales) = app.new_user("SALES", PASSWORD).await;
    let c = app.as_user(&sales).await;
    let first = c.get("/api/teams?size=2").await.body;
    let second = c
        .get(&format!(
            "/api/teams?size=2&cursor={}",
            first["nextCursor"].as_str().unwrap()
        ))
        .await
        .body;
    assert!(ids(&second)[0] > ids(&first)[1]);
}

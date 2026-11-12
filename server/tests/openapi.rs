//! The OpenAPI document keeps the names the web app's generated client uses (orval: tags-split, operationId
//! = function name, schema name = model name), so `yarn api:gen` does not rename anything.

mod support;

use support::*;

#[tokio::test]
async fn operation_ids_tags_and_schemas_match_the_web_client() {
    let app = TestApp::start().await;
    let doc = app.client().get("/v3/api-docs").await;
    assert_eq!(doc.status, 200);
    let paths = doc.body["paths"].as_object().unwrap();
    let mut ops = Vec::new();
    for (path, item) in paths {
        for (method, op) in item.as_object().unwrap() {
            ops.push((
                format!("{} {path}", method.to_uppercase()),
                op["operationId"].as_str().unwrap().to_owned(),
                op["tags"][0].as_str().unwrap_or_default().to_owned(),
            ));
        }
    }
    let expect = [
        ("GET /api/auth/csrf", "getCsrfToken", "Auth"),
        ("POST /api/auth/login", "login", "Auth"),
        ("GET /api/auth/dev-login", "getDevLoginStatus", "Auth"),
        ("POST /api/auth/dev-login", "devLogin", "Auth"),
        ("GET /api/auth/me", "getMe", "Auth"),
        ("POST /api/auth/password", "changeOwnPassword", "Auth"),
        ("GET /api/users", "listUsers", "Users"),
        ("POST /api/users", "createUser", "Users"),
        ("PUT /api/users/{id}/role", "changeUserRole", "Users"),
        ("PUT /api/users/{id}/active", "setUserActive", "Users"),
        ("PUT /api/users/{id}/password", "resetUserPassword", "Users"),
        ("GET /api/clients", "searchClients", "Clients"),
        ("POST /api/clients", "createClient", "Clients"),
        ("GET /api/clients/count", "countClients", "Clients"),
        ("GET /api/clients/{id}", "getClient", "Clients"),
        ("PUT /api/clients/{id}", "updateClient", "Clients"),
        ("DELETE /api/clients/{id}", "deleteClient", "Clients"),
        ("GET /api/teams", "listTeams", "Teams"),
        ("POST /api/teams", "createTeam", "Teams"),
        ("GET /api/teams/{id}/members", "listTeamMembers", "Teams"),
        ("POST /api/teams/{id}/members", "addTeamMember", "Teams"),
        (
            "DELETE /api/teams/{id}/members/{userId}",
            "removeTeamMember",
            "Teams",
        ),
        ("GET /api/teams/{id}/stats", "getTeamStats", "Teams"),
        ("GET /api/projects", "searchProjects", "Projects"),
        ("POST /api/projects", "createProject", "Projects"),
        ("GET /api/projects/count", "countProjects", "Projects"),
        ("GET /api/projects/{id}", "getProject", "Projects"),
        ("PUT /api/projects/{id}", "updateProject", "Projects"),
        (
            "POST /api/projects/{id}/stage",
            "changeProjectStage",
            "Projects",
        ),
        (
            "GET /api/projects/{id}/stages",
            "listProjectStages",
            "Projects",
        ),
        (
            "GET /api/projects/{id}/comments",
            "listProjectComments",
            "Projects",
        ),
        (
            "POST /api/projects/{id}/comments",
            "addProjectComment",
            "Projects",
        ),
        (
            "GET /api/desk/reports/categories/{slug}",
            "getCategoryReport",
            "Desk reports",
        ),
        (
            "GET /api/desk/reports/categories/{slug}/export.{format}",
            "exportCategoryReport",
            "Desk reports",
        ),
        (
            "GET /api/desk/reports/agents",
            "getAgentReport",
            "Desk reports",
        ),
        (
            "GET /api/desk/reports/agents/export.{format}",
            "exportAgentReport",
            "Desk reports",
        ),
        ("GET /api/desk/charts", "getDeskCharts", "Desk charts"),
    ];
    for (route, id, tag) in expect {
        let found = ops.iter().find(|(r, _, _)| r == route);
        assert_eq!(
            found.map(|(_, i, t)| (i.as_str(), t.as_str())),
            Some((id, tag)),
            "{route}"
        );
    }
    let schemas = doc.body["components"]["schemas"].as_object().unwrap();
    for name in [
        "LoginRequest",
        "MeResponse",
        "DevLoginRequest",
        "DevLoginStatus",
        "ChangePasswordRequest",
        "UserResponse",
        "PageResponseUserResponse",
        "CreateUserRequest",
        "ChangeRoleRequest",
        "SetActiveRequest",
        "ResetPasswordRequest",
        "ReportTableResponse",
        "ReportColumnResponse",
        "StreamingResponseBody",
        "ChartData",
        "FlowPoint",
        "KeyFigures",
        "SlaUse",
        "Breakdown",
    ] {
        assert!(
            schemas.contains_key(name),
            "schema {name} missing: {:?}",
            schemas.keys()
        );
    }
    let required = doc.body["components"]["schemas"]["UserResponse"]["required"]
        .as_array()
        .unwrap();
    assert!(required.iter().any(|r| r == "rowVersion"));
    assert!(!required.iter().any(|r| r == "lockedUntil"));
    let budget = &doc.body["components"]["schemas"]["ProjectResponse"]["properties"]["budget"];
    assert_eq!(
        budget["type"], "number",
        "budget is a number for the web client: {budget}"
    );
    let search = &doc.body["paths"]["/api/projects"]["get"]["parameters"];
    let names: Vec<_> = search
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "stage",
            "teamId",
            "clientId",
            "ownerUserId",
            "overdue",
            "dueFrom",
            "dueTo",
            "cursor",
            "size"
        ]
    );
    let public = &doc.body["paths"]["/api/public/desk/tickets"]["post"]["requestBody"]["content"];
    assert!(
        public.get("multipart/form-data").is_some(),
        "the public form is multipart: {public}"
    );
    assert!(
        !schemas.contains_key("ChartRange"),
        "the range stays inline (ChartDataRange, GetDeskChartsRange in the web client)"
    );
    let range = &doc.body["paths"]["/api/desk/charts"]["get"]["parameters"][0];
    assert_eq!(range["name"], "range");
    assert_eq!(
        range["schema"]["enum"],
        serde_json::json!(["DAYS_7", "DAYS_30", "WEEKS_12"])
    );
    assert_eq!(app.client().get("/swagger-ui/").await.status, 200);
}

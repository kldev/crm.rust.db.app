# Diagrams

## Architecture

The application never touches a table. Every endpoint is hand-written in Rust and calls one `CALL app.<procedure>`
or reads one `SELECT ... FROM app.<function|view>` with a login that has only `EXECUTE` / `SELECT` on schema `app`.

```mermaid
flowchart LR
    client["Browser: React + HeroUI<br/>(static files from the same binary)<br/>or curl, Swagger UI, k6"]
    subgraph app["Rust binary crm (axum, tokio-postgres)"]
        security["session cookie + CSRF<br/>(tower-sessions)"]
        handlers["handlers<br/>(hand-written endpoints, utoipa)"]
        sql["one statement each<br/>CALL app.* / SELECT ... FROM app.*"]
        errors["ApiError<br/>SQLSTATE U1xxx -> ProblemDetail 51xxx"]
    end
    subgraph db["PostgreSQL 18"]
        appschema["schema app<br/>procedures, functions, views<br/>(SECURITY DEFINER)"]
        crmschema["schema crm<br/>tables, internal helpers"]
        seedschema["schema seed<br/>seed.run, seed.desk_run"]
    end
    s3[("RustFS (S3)<br/>desk attachments")]
    client -->|HTTPS JSON| security --> handlers --> sql
    sql -->|"login crm_app"| appschema
    appschema -->|"runs as crm_owner"| crmschema
    sql -. SQL error .-> errors
    handlers --> s3
    seedschema --> crmschema
```

## Containers (docker compose)

```mermaid
flowchart TB
    pg[("postgres<br/>PostgreSQL 18.6")]
    rustfs[("rustfs<br/>S3, bucket by rustfs-init")]
    init["db-init (one-shot)<br/>CRM_MODE=init: roles, database"]
    migrate["migrate (one-shot)<br/>CRM_MODE=migrate: db/migration V* and R__*, login crm_migrator"]
    app["app<br/>CRM_MODE=app, login crm_app"]
    seed["seed (profile seed, one-shot)<br/>CRM_MODE=seed: CALL seed.run, seed.desk_run"]
    k6["k6 (profile perf)<br/>results -> perf/results/"]
    pg -->|healthy| init -->|completed| migrate -->|completed| app
    rustfs -->|healthy| app
    migrate -->|completed| seed
    app -->|healthy| k6
```

## Data model

```mermaid
erDiagram
    USERS ||--o{ CLIENTS : "is assigned"
    USERS ||--o{ TEAMS : "leads"
    TEAMS ||--o{ TEAM_MEMBERS : has
    USERS ||--o{ TEAM_MEMBERS : "is member"
    CLIENTS ||--o{ PROJECTS : "orders"
    TEAMS ||--o{ PROJECTS : delivers
    USERS ||--o{ PROJECTS : owns
    PROJECTS ||--o{ PROJECT_STAGE_HISTORY : "moves through"
    PROJECTS ||--o{ PROJECT_COMMENTS : has
    USERS ||--o{ PROJECT_COMMENTS : writes
    PROJECT_STAGE_TRANSITIONS }o--o{ PROJECTS : "allows stage changes of"

    USERS {
        int id PK
        text login UK
        text display_name
        text role "ADMIN | MANAGER | SALES"
        text password_hash
        boolean is_active
        int failed_login_count
        timestamptz locked_until
        xid xmin "rowVersion"
    }
    CLIENTS {
        bigint id PK
        text name
        text nip UK "10 digits"
        text email
        text city
        text status "LEAD | ACTIVE | INACTIVE"
        int assigned_user_id FK
        xid xmin "rowVersion"
    }
    TEAMS {
        int id PK
        text name UK
        int lead_user_id FK "MANAGER"
    }
    TEAM_MEMBERS {
        int team_id PK, FK
        int user_id PK, FK
    }
    PROJECTS {
        bigint id PK
        bigint client_id FK
        int team_id FK
        int owner_user_id FK "member of the team"
        text name
        text stage
        date start_date
        date due_date
        numeric budget
        xid xmin "rowVersion"
    }
    PROJECT_STAGE_TRANSITIONS {
        text from_stage PK
        text to_stage PK
    }
    PROJECT_STAGE_HISTORY {
        bigint id PK
        bigint project_id FK
        text from_stage
        text to_stage
        int changed_by
    }
    PROJECT_COMMENTS {
        bigint id PK
        bigint project_id FK
        int author_id FK
        text body
    }
```

`rowVersion` is the row's `xmin`. Read model: `crm.team_project_stats` (count and budget per team and stage), kept
in step by the procedures that write projects (`crm.team_stats_move`) - PostgreSQL has no indexed views.

## Service desk

One event stream per ticket: every action appends one row to `crm.ticket_events` (version = the next number of the
ticket) and updates the read model `crm.tickets` in the same transaction; the ticket's row lock decides between two
agents acting at once (U1910 -> 51910 for the slower one). Report data and attachments are events too, with their
current state in `crm.ticket_fields` and `crm.ticket_attachments`; the files themselves are in S3.

```mermaid
erDiagram
    DESK_CATEGORIES ||--o{ DESK_SUBCATEGORIES : has
    DESK_SUBCATEGORIES ||--o{ DESK_SUBCATEGORY_FIELDS : collects
    DESK_FIELDS ||--o{ DESK_SUBCATEGORY_FIELDS : "is used by"
    DESK_FIELDS ||--o{ DESK_FIELD_OPTIONS : offers
    DESK_PRIORITIES ||--o{ TICKETS : "sets the SLA of"
    DESK_SUBCATEGORIES ||--o{ TICKETS : classifies
    PEOPLE ||--o{ TICKETS : "report / do the work"
    USERS ||--o{ TICKETS : "work on (agents)"
    TICKETS ||--o{ TICKET_EVENTS : "stream"
    TICKETS ||--o{ TICKET_FIELDS : "report data now"
    TICKETS ||--o{ TICKET_ATTACHMENTS : "files now"

    TICKETS {
        bigint id PK
        text number UK "HAL- + 8 hex"
        text status "IN_POOL | IN_PROGRESS | DONE"
        text priority
        timestamptz reported_at
        timestamptz deadline "reported_at + SLA"
        int assignee_user_id
        boolean resolved_late
        int version "last event folded in"
    }
    TICKET_EVENTS {
        bigint ticket_id PK, FK
        int version PK
        text event_type "TicketReported, TicketTaken, ..."
        timestamptz at
        int actor_user_id
        jsonb data
    }
    TICKET_FIELDS {
        bigint ticket_id PK, FK
        text field_key PK, FK
        text value "normal form"
    }
    TICKET_ATTACHMENTS {
        uuid attachment_id PK
        bigint ticket_id FK
        text file_name
        text content_type
        bigint size_bytes
        text storage_key UK "attachments/<uuid> in the bucket"
    }
```

```mermaid
stateDiagram-v2
    [*] --> IN_POOL: reported (agent or public form)
    IN_POOL --> IN_PROGRESS: take
    IN_PROGRESS --> IN_PROGRESS: hand over
    IN_PROGRESS --> IN_POOL: return
    IN_PROGRESS --> DONE: resolve (late when at or after the deadline)
    DONE --> [*]
```

An upload: `app.desk_attachment_check` checks every file before a byte is stored, the bytes go to the bucket, then
one procedure appends the events; when the database refuses, the stored objects are deleted again.

```mermaid
sequenceDiagram
    autonumber
    participant C as Client
    participant A as attach handler (Rust)
    participant D as PostgreSQL
    participant S as RustFS (S3)
    C->>A: POST /api/desk/tickets/42/attachments (multipart, up to 10 x 20 MB)
    A->>D: SELECT ... FROM app.desk_attachment_check(name, size) for each file
    A->>S: PutObject attachments/<uuid> for each file
    A->>D: CALL app.desk_ticket_attach(actor, 42, files)
    alt refused (e.g. 51124: more than 10 per ticket)
        A->>S: DeleteObject of the stored files
        A-->>C: 400 ProblemDetail {errorNumber: 51124}
    else ok
        D-->>A: TicketAttachmentAdded events, crm.ticket_attachments rows
        A-->>C: 200 the attachments
    end
```

## Project stages

The allowed moves are rows of `crm.project_stage_transitions`; `app.project_change_stage` refuses anything else
with SQLSTATE U1902, answered as error 51902 (HTTP 409).

```mermaid
stateDiagram-v2
    [*] --> NEW
    NEW --> ACCEPTED
    NEW --> DECLINED
    ACCEPTED --> ACTIVE
    ACCEPTED --> DECLINED
    ACTIVE --> ON_HOLD
    ON_HOLD --> ACTIVE
    ACTIVE --> CLOSED
    ON_HOLD --> CLOSED
    CLOSED --> [*]
    DECLINED --> [*]
```

## Login

```mermaid
sequenceDiagram
    autonumber
    participant C as Client
    participant S as session + CSRF layer
    participant A as auth handler (Rust)
    participant D as PostgreSQL
    C->>S: GET /api/auth/csrf
    S-->>C: 204 + cookie XSRF-TOKEN
    C->>A: POST /api/auth/login {login, password} + header X-XSRF-TOKEN
    A->>D: SELECT ... FROM app.user_get_for_login($1)
    D-->>A: id, role, password_hash, is_active, locked_until
    A->>A: bcrypt verify (blocking pool), locked / disabled checks
    alt wrong password
        A->>D: CALL app.user_login_failed (locks after N failures)
        A-->>C: 401 "Invalid login or password." (same for every failure)
    else success
        A->>D: CALL app.user_login_succeeded
        A->>A: new session id (cycle), actor in the session
        A-->>C: 200 {id, login, role} + session cookie
    end
```

## A write with a business error

```mermaid
sequenceDiagram
    autonumber
    participant C as Client
    participant P as change_project_stage (Rust handler)
    participant R as db::query_one
    participant D as app.project_change_stage
    participant H as ApiError
    C->>P: POST /api/projects/42/stage {stage: CLOSED, rowVersion}
    P->>R: CALL app.project_change_stage($1..$5)
    R->>D: actor_id, id, to_stage, comment, row_version
    D->>D: actor active? project visible? row_version current? transition allowed?
    alt rule broken
        D-->>R: RAISE ... USING ERRCODE = 'U1902'
        R-->>H: tokio_postgres error, SQLSTATE U1902
        H-->>C: 409 ProblemDetail {detail, errorNumber: 51902}
    else ok
        D->>D: UPDATE projects, INSERT stage history (+ comment), one transaction
        D-->>R: the project row
        R-->>C: 200 ProjectResponse (new rowVersion)
    end
```

## Performance test

```mermaid
flowchart LR
    seed["./start.sh --seed<br/>CALL seed.run<br/>5M clients, 5M projects,<br/>2M comments, 5000 users"]
    k6["./start.sh --k6<br/>200 VUs: login, search,<br/>details, writes"]
    reads["perf/sql: shared buffers of each function,<br/>pg_stat_statements of the run"]
    results["perf/results/&lt;date&gt;-&lt;scenario&gt;/<br/>summary.json, summary.txt, REPORT.md"]
    seed --> k6 --> results
    seed --> reads --> results
```

# R CRM (crm.rust.db.app)

A small CRM: users, clients, teams and the projects that teams deliver for clients, with stages and comments - and a
**service desk** (ported from Halcyon): requests to the company's teams, one event stream per ticket. **Rust**
(axum, tokio-postgres) for the HTTP API and identity, **all data logic in PL/pgSQL procedures and SQL functions** on
**PostgreSQL 18**. No ORM, no query mapper, no generated endpoints: every endpoint is written by hand and calls
exactly one `CALL app.<procedure>` or reads one `SELECT ... FROM app.<function|view>`.

The project started as Kotlin + Spring Boot on SQL Server 2025 Express and was rewritten step by step with the same
HTTP contract (paths, JSON fields, error numbers); the web app did not change.

- Diagrams: [docs/diagrams.md](docs/diagrams.md)
- Performance: [perf/README.md](perf/README.md)

### Why Rust

`docker stats` right after `./start.sh` (2026-11-12, idle, nobody signed in):

```text
NAME             CPU %     MEM USAGE / LIMIT     PIDS
crm-app-1        0.00%     6.508MiB / 30.46GiB   17
crm-postgres-1   0.00%     113.7MiB / 30.46GiB   10
crm-rustfs-1     0.03%     94.54MiB / 30.46GiB   52
```

- **About 6.5 MiB at start.** The whole API (HTTP, sessions, OpenAPI, S3 client, XLSX/PDF exports and the web app's
  files) is one native binary with no runtime to warm up, so it takes less memory than the database's idle
  connections. Under 200 virtual users it peaked at about 36 MiB; the Kotlin / Spring Boot version on the JVM held
  about 480 MiB for the same traffic ([perf/README.md](perf/README.md)).
- **Ready in about 0.1 s.** From the container start to `listening port=8080` in the log took 0.12 s, so a restart
  after a migration or a new release is not noticeable, and the healthcheck passes on its first try.
- **Low CPU under load.** 4% on average (13% max) at 339 requests/s; the latency is the database's, not the API's.
- **Fewer failure modes at run time.** No GC pauses, no reflection or classpath scanning, errors from the database
  mapped by hand in `error.rs`; what the compiler accepts is what runs.
- **Cheap to host.** The API fits next to PostgreSQL on the smallest VPS; memory goes to the database's cache, where
  it pays off.

## Web app

A React app (Vite, HeroUI v3, TanStack Router/Query/Form) in `frontend/`, styled with the Ambit Obsidian theme,
served by the same binary: open <http://127.0.0.1:8088/> after `./start.sh`.

- Overview (overdue and due-soon projects, latest clients), clients with prefix search and filters in the URL,
  client details with its projects, projects with stage, team and overdue filters, project details with the stage
  path, stage changes with a comment, comments and stage history, teams with delivery statistics and membership,
  users (ADMIN/MANAGER).
- Service desk: the pool, my tickets and all tickets (filters in the URL, open by deadline then resolved, total on
  request), counters of overdue / due soon / pool / mine in the top bar, a new ticket registered by an agent (dated
  back up to 30 days), the ticket page (take, resolve, hand over, return, priority with the new deadline, the person
  doing the work, comments, attachments with image thumbnails, the timeline from the event stream), agents and
  people. The public form <http://127.0.0.1:8088/report> needs no sign-in, takes files with the report and gives
  the requester the ticket number. Attachments live in RustFS (S3, console <http://127.0.0.1:19001>).
- Report data per topic (PESEL with its date of birth, IBAN, amounts, dates, choices) on the forms and the ticket
  page, five category reports and the agents' work with XLSX, PDF and CSV exports, campaigns linked to recruitment
  tickets.
- Charts: flow in and out, backlog, deadlines kept, SLA use per priority, by category, the queue now, agents and a
  weekday x hour heatmap, each with a table view. The API gives the numbers, the browser draws them.
- Polish and English with a PL / EN switch; Polish is the default.

## How it works

| Layer | What it does |
|---|---|
| `server/src/auth`, `web.rs` | login (JSON, bcrypt), session cookie (`tower-sessions`), CSRF cookie + header, health, the web app's files |
| Handlers (`server/src/{users,clients,teams,projects,desk,reports,charts}.rs`) | request checks, paging cursors, one statement: `CALL app.*` or `SELECT ... FROM app.*`; rows mapped by hand (`row.get("column")`); OpenAPI written with `utoipa` |
| `server/src/error.rs` | SQLSTATE `U1xxx` from the database -> RFC 9457 ProblemDetail with `errorNumber` 51xxx |
| `server/src/exports.rs` | report files: CSV by hand, XLSX (`rust_xlsxwriter`), PDF (`krilla`, Lato embedded) |
| `server/src/storage.rs` | desk attachments in S3 (`aws-sdk-s3`, path-style, RustFS locally) |
| schema `app` (`db/migration/R__app_*.sql`) | every rule: validation, visibility, state changes, concurrency (`xmin` as `rowVersion`); `SECURITY DEFINER`, owned by `crm_owner` |
| schema `crm` (`V*__*.sql`, `R__crm_*.sql`) | tables, internal helpers, the desk's event stream `crm.ticket_events` and its read model `crm.tickets`, `crm.team_project_stats` kept in step by the procedures |
| schema `seed` (`R__seed_*.sql`) | the one data seed (`CRM_MODE=seed`) |

One binary `crm`, four modes (`CRM_MODE`): `app` (the API), `init` (roles and database, superuser), `migrate`
(`db/migration`: `V<n>__*.sql` once and in order, `R__*.sql` again when changed; embedded in the binary) and `seed`.

The application's login `crm_app` has **only** `USAGE` on schema `app`, `EXECUTE` on its routines and `SELECT` on its
views; no table, no other schema. No Row Level Security: visibility (a SALES user sees own clients) is decided by
the procedures and functions from `p_actor_id`. `cargo test` fails if an ORM or query mapper reaches the dependencies
or if a statement in the API code is anything but `CALL app.*` / `SELECT ... FROM app.*`.

## Local development

### Requirements

| Tool | Needed for |
|---|---|
| Docker with compose v2 | the whole stack (PostgreSQL, RustFS, migrations, app) - enough to just run the app; also the tests (testcontainers) |
| Rust 1.99.1 (rustup, with `rustfmt` and `clippy`) | `cargo test`, `cargo run` of the API from the IDE |
| Node 22+ and corepack (Yarn 4.18 from `packageManager`) | front-end development (`frontend/`) |

### 1. First start (everything in Docker)

```bash
cd infrastructure
cp .env-sample .env   # fill in every empty password
./start.sh            # postgres -> db-init -> migrate -> app; rustfs -> rustfs-init (bucket) -> app
```

Then open:

- web app: <http://127.0.0.1:8088/> (port `CRM_HTTP_PORT`)
- API docs: <http://127.0.0.1:8088/swagger-ui.html>, OpenAPI: `/v3/api-docs`
- PostgreSQL: `127.0.0.1:15432` (`CRM_DB_HOST_PORT`), database `crm`, superuser `postgres` / `POSTGRES_PASSWORD`
  (DataGrip, psql), or `docker compose --env-file .env exec postgres psql -U postgres -d crm`

Log in as `CRM_ADMIN_LOGIN` / `CRM_ADMIN_PASSWORD` (created on the first start). The database starts empty.

### 2. Test data (optional)

```bash
CRM_SEED_CLIENTS=10000 CRM_SEED_PROJECTS=10000 CRM_SEED_COMMENTS=5000 ./start.sh --seed   # small, seconds
./start.sh --seed                                                                         # full set, about 6 min
```

After the seed log in as `perf.sales.0001`, `perf.manager.0001` or `perf.admin.0001` with `CRM_SEED_PASSWORD`
(or with one click on the login page while `CRM_DEV_LOGIN=true`). The seed runs only on an empty database (error
51060 otherwise); start over with `./start.sh --clean --seed`. It also seeds the service desk (`seed.desk_run`: ten
agents such as `anna.nowak`, 40 people, `CRM_SEED_DESK_TICKETS` tickets). That part is idempotent; for a database
seeded before the desk existed run only it:
`CRM_SEED_DESK_ONLY=true docker compose --env-file .env --profile seed run --rm seed`.

### 3. Day to day

| Task | Command |
|---|---|
| rebuild and restart after a Rust or SQL change (migrations run again) | `infrastructure/start.sh` |
| stop everything (data stays in the `pg-data` volume) | `infrastructure/start.sh --down` |
| drop the database and start from zero | `infrastructure/start.sh --clean` |
| tests (unit, architecture guard, HTTP and SQL on PostgreSQL 18 in testcontainers) | `cd server && cargo test` |
| one test file | `cd server && cargo test --test desk_http` |
| format and lint | `cd server && cargo fmt && cargo clippy --all-targets -- -D warnings` |
| front-end lint and format | `cd frontend && yarn check` / `yarn format` |

### Front end with hot reload

Keep the stack running (`./start.sh`), then:

```bash
cd frontend
corepack enable
yarn install
yarn dev              # http://127.0.0.1:5173, /api and /v3 proxied to http://127.0.0.1:8088
```

The proxy keeps one origin, so the session and `XSRF-TOKEN` cookies work as in production. A different API
address: `CRM_API_URL=http://127.0.0.1:8080 yarn dev`. After changing an endpoint, regenerate the client with the
new API running: `yarn api:gen` (orval from `/v3/api-docs`, files in `src/api/`).

### API from the IDE (`cargo run`)

Run the database and migrations in Docker, the API from RustRover / VS Code or the terminal:

```bash
cd infrastructure && ./start.sh && docker compose --env-file .env stop app && cd ..
set -a; . infrastructure/.env; set +a
export CRM_DB_HOST=127.0.0.1 CRM_DB_PORT=$CRM_DB_HOST_PORT CRM_S3_ENDPOINT=http://127.0.0.1:19000 PORT=8088
cd server && cargo run
```

In the IDE set the same variables in the run configuration. With `PORT=8088` `yarn dev` works without changes.
Migrations are applied by the `migrate` container (or `CRM_MODE=migrate cargo run`), so after a SQL change run
`infrastructure/start.sh` again. To serve the built web app from `cargo run`, run `frontend/build-to-resources.sh`
first: it copies `dist/` to `server/static` (git-ignored), the directory the binary serves (`CRM_STATIC_DIR`,
default `static`).

### Calling the API with curl

```bash
B=http://127.0.0.1:8088
curl -s -c j -b j -o /dev/null $B/api/auth/csrf
X=$(awk '$6=="XSRF-TOKEN"{print $7}' j)
curl -s -c j -b j -H "X-XSRF-TOKEN: $X" -H 'Content-Type: application/json' \
     -d '{"login":"perf.sales.0001","password":"<CRM_SEED_PASSWORD>"}' $B/api/auth/login
curl -s -b j "$B/api/clients?namePrefix=Zielony&size=5"
curl -s -b j "$B/api/projects?overdue=true&size=5"
```

## Endpoints

Every name below is in schema `app`: a procedure (`CALL`) for writes, a function or view (`SELECT ... FROM`) for
reads. A write that answers with the changed row reads it in the same transaction (`*_get`).

| Method and path | PostgreSQL | Who |
|---|---|---|
| `GET /api/auth/csrf` | - | anyone |
| `POST /api/auth/login` | `user_get_for_login`, `user_login_succeeded` / `user_login_failed` | anyone |
| `GET/POST /api/auth/dev-login` | `user_get_for_login`, `user_login_succeeded` (only with `CRM_DEV_LOGIN=true`) | anyone |
| `POST /api/auth/logout`, `GET /api/auth/me` | - (the session) | logged in |
| `POST /api/auth/password` | `user_get_for_login`, `user_set_password` | logged in |
| `GET /api/users` | `user_list` | ADMIN, MANAGER |
| `POST /api/users`, `PUT /api/users/{id}/role`, `/active`, `/password` | `user_create`, `user_change_role`, `user_set_active`, `user_set_password` | ADMIN |
| `GET /api/clients`, `/count` | `client_search`, `client_count` | all (SALES: own clients) |
| `GET/POST/PUT/DELETE /api/clients[/{id}]` | `client_get`, `client_create`, `client_update`, `client_delete` | all; delete: MANAGER, ADMIN |
| `GET /api/projects`, `/count` | `project_search`, `project_count` (filters: stage, team, client, owner, overdue, due range) | all (SALES: own / team / client projects) |
| `GET/POST/PUT /api/projects[/{id}]` | `project_get`, `project_create`, `project_update` | all visible |
| `POST /api/projects/{id}/stage` | `project_change_stage` | all visible |
| `GET /api/projects/{id}/stages` | `project_stage_history_list` | all visible |
| `GET/POST /api/projects/{id}/comments` | `project_comment_list`, `project_comment_add` | all visible |
| `GET/POST /api/teams`, `GET /api/teams/{id}/members`, `GET /api/teams/{id}/stats` | `team_list`, `team_create`, `team_member_list`, `team_project_stats` | all; create: ADMIN |
| `POST/DELETE /api/teams/{id}/members[/{userId}]` | `team_member_add`, `team_member_remove` | ADMIN, team lead |
| `GET /api/desk/catalog` | views `v_desk_catalog`, `v_desk_fields`, `v_desk_field_options`, `v_desk_priorities`, `v_desk_attachment_types` | all |
| `GET /api/desk/tickets`, `/count`, `/counters` | `desk_ticket_search`, `desk_ticket_count`, `desk_ticket_counters` | all |
| `POST /api/desk/tickets`, `GET /api/desk/tickets/{id}`, `/by-number/{number}`, `/{id}/events` | `desk_ticket_report`, `desk_ticket_get`, `desk_ticket_find`, `desk_ticket_event_list` | all |
| `POST /api/desk/tickets/{id}/take`, `/hand-over`, `/return`, `/priority`, `/resolve`, `/comments`, `/handler` | `desk_ticket_take`, `_hand_over`, `_return`, `_change_priority`, `_resolve`, `_comment`, `_assign_handler` | all (the rules of the assignee in the procedures) |
| `GET/POST /api/desk/tickets/{id}/attachments`, `GET …/attachments/{attachmentId}` | `desk_attachment_list`, `desk_attachment_check` + `desk_ticket_attach`, `desk_attachment_get` | all |
| `GET/PUT /api/desk/tickets/{id}/fields`, `GET …/linked` | `desk_ticket_field_list`, `desk_ticket_update_fields`, `desk_ticket_linked_list` | all |
| `GET /api/desk/reports/categories/{slug}[/export.{csv,xlsx,pdf}]` | `desk_report_category` (+ the catalog views for the column names) | all |
| `GET /api/desk/reports/agents[/export.{csv,xlsx,pdf}]` | `desk_report_agents`, `desk_report_period_list` | all |
| `GET /api/desk/charts?range=DAYS_7\|DAYS_30\|WEEKS_12&categoryKey=` | `desk_chart_tickets` | all |
| `GET /api/desk/agents`, `/people` | `desk_agent_list`, `desk_person_list` | all |
| `GET /api/public/desk/catalog`, `/people`, `POST /api/public/desk/tickets` | the catalog views, `v_desk_public_people`, `desk_attachment_check` + `desk_ticket_report_public` | anyone (public form, 30 tickets/min) |
| `GET /actuator/health[/readiness\|/liveness]` | `health_ping`, S3 `HeadBucket` | anyone |

Lists are keyset-paged: pass `nextCursor` from a page as `cursor` for the next one. Updates need the
`rowVersion` from the last read (409 when someone changed the row in the meantime).

## Error catalogue

Procedures raise SQLSTATE `U1xxx` (`RAISE ... USING ERRCODE = 'U1902'`); the API answers with RFC 9457 ProblemDetail
and `errorNumber` 51xxx - the numbers the T-SQL version used, so the web app's messages
(`frontend/src/i18n/problems.ts`) did not change.

| Number | Meaning | HTTP |
|---|---|---|
| 50001 | actor not passed (bug) | 500 |
| 51010-51013 | client: name, NIP (10 digits), e-mail, status | 400 |
| 51014 | page size outside 1-200 | 400 |
| 51015, 51016, 51019, 51020 | user: login, role, display name, password | 400 |
| 51017 | client can be assigned only to an active sales user | 400 |
| 51018 | cannot change own role or deactivate yourself | 400 |
| 51030-51034 | project: name, stage, dates, budget, comment text | 400 |
| 51035-51038 | team: lead must be an active manager, name, owner must be a team member, member must be active | 400 |
| 51050, 51060 | seed: invalid parameter, database already seeded | 400 |
| 51100-51110 | desk ticket: title, description, category, subcategory (retired), priority, report date (future, > 30 days back), requester | 400 |
| 51111-51116 | desk: reason, comment, hand over to yourself, handler outside the department, list filter | 400 |
| 51120-51124 | attachment: no name, type not allowed, empty, over 20 MB, more than 10 per ticket | 400 |
| 51130-51146 | report data: field of another subcategory, required, length, date, e-mail, amount, IBAN, count, multi-choice, choice, ticket number, identity document and PESEL rules (check digit and date of birth), no report data | 400 |
| 51150 | report period: from <= to, at most a year | 400 |
| 51301 | role not allowed | 403 |
| 51302 | own account not active | 403 |
| 51303 | sales user works only on own clients | 403 |
| 51310-51313 | only the agent working on the ticket can hand it over, return it, resolve it, name its handler | 403 |
| 51401-51404 | client, user, project, team not found (or not visible) | 404 |
| 51410-51413 | ticket, person, agent, attachment not found | 404 |
| 51901 | `rowVersion` changed by someone else | 409 |
| 51902 | stage change not allowed / closed project | 409 |
| 51904 | client has projects, cannot be deleted | 409 |
| 51905 | member still owns open projects of the team | 409 |
| 51910 | ticket already in progress with another agent (someone was faster) | 409 |
| 51911, 51912 | ticket already resolved / still in the pool (take it first) | 409 |
| 51913 | the ticket's stream changed meanwhile | 409 |
| 2627 | duplicate (login, NIP, team name): SQLSTATE 23505 | 409 |

## PostgreSQL 18

`postgres:18.6` in `infrastructure/docker-compose.yml`, with the settings stated in the file (each with its reason):
`shared_buffers=2GB`, `effective_cache_size=8GB`, `work_mem=16MB`, `maintenance_work_mem=1GB`, `max_wal_size=8GB`,
`random_page_cost=1.1`, `max_connections=100`, `pg_stat_statements` preloaded (`track=all`) and
`track_functions=pl` for `perf/sql`. Roles (`db/init/01_roles.sql`):

| Role | Login | What it may do |
|---|---|---|
| `crm_owner` | no | owns every object of `crm`, `app`, `seed`; `app` routines run as it (`SECURITY DEFINER`) |
| `crm_migrator` | yes | member of `crm_owner`: migrations and the seed (`CRM_MODE=migrate\|seed`) |
| `crm_app` | yes | the API: schema `app` only |

## Seed

One seed (`seed.run`, `seed.desk_run`), default values below, each configurable in `infrastructure/.env`
(`CRM_SEED_*`). Set-based (`INSERT ... SELECT` over `generate_series` in batches of `CRM_SEED_BATCH_SIZE`), one
transaction, the secondary indexes built after the load, then `ANALYZE`. Deterministic for the same values and
`CRM_SEED_RANDOM_SEED`: every value comes from `sha256('<seed>:<kind>:<n>')`, the same numbers as the T-SQL seed, so
the PostgreSQL and SQL Server databases hold the same data.

| Data | Default |
|---|---|
| Users | 5 000: 10 ADMIN, 490 MANAGER, 4 500 SALES |
| Teams | 900 (5 SALES each, led by managers) |
| Clients | 5 000 000 |
| Projects | 5 000 000 (+ 5 000 000 stage history rows) |
| Comments | 2 000 000 |
| Desk | 10 agents, 40 people, 200 tickets |

Measured with the defaults (2026-11-12, laptop: 16 cores, 30 GB RAM, Docker; PostgreSQL 18.6):

| Measure | PostgreSQL 18.6 | SQL Server 2025 Express (2026-11-08) |
|---|---:|---:|
| Seed time | **375 s** (clients 109 s, projects 187 s, history, team statistics and comments 52 s, 14 indexes 27 s) | 798 s |
| Database size | 5.4 GB (tables 1.9 GB, indexes 3.5 GB) | 7.6 GB files (data 2.4 GB, indexes 3.5 GB) |
| Largest tables (table + indexes) | projects 0.7 + 1.8 GB, clients 0.8 + 1.3 GB, stage history 0.3 + 0.3 GB, comments 0.2 + 0.1 GB | projects 2.5 GB, clients 2.4 GB, stage history 0.5 GB, comments 0.4 GB |

## Performance

200 virtual users of mixed traffic for 13 minutes on the full data set, everything on one laptop:

| Run | p95 | p99 | Errors | Requests/s | DB CPU avg |
|---|---:|---:|---:|---:|---:|
| Kotlin + SQL Server 2025 Express, after the read indexes (2026-11-08) | 8.5 ms | 10.4 ms | 0.00% | 338 | 58% |
| **Rust + PostgreSQL 18.6** (2026-11-12) | **4.63 ms** | **5.52 ms** | 0.00% | 339 | 27% |

The same data, scenario and laptop; Express was capped at 4 cores and a 1.4 GB buffer pool, PostgreSQL was not,
but neither database was the limit (CPU far below the cap). 100% CPU = one core.

Details, per-endpoint latencies, buffers per function and the differences between the environments:
[perf/README.md](perf/README.md).

## Tests

`cd server && cargo test` runs everything on a real PostgreSQL 18.6 (testcontainers, one container per test file)
and, for attachments, a real RustFS: architecture rules (only `CALL app.*` / `SELECT ... FROM app.*` in the API code,
no ORM or query mapper), database permissions (the app login cannot read a table, create objects or reach `crm`,
`seed` and the migration history), idempotent migrations, login, lockout and dev login, users, clients, projects,
stages, comments and team statistics over HTTP, the OpenAPI names the web client uses, the service desk (rules of
every action, three agents taking one ticket at once, deadlines, lists, the public form and its limit), attachments
(refused types, the limit of 10, refused uploads deleted from the bucket, download headers), report data (every kind,
PESEL and IBAN on a table of cases, changes listed in the event), the reports and their CSV / XLSX / PDF exports
(the CSV parses, the XLSX has the same rows, the PDF holds Polish text), the charts' numbers agreeing with each other
and the lists, and the seed (counts, the T-SQL hash numbers, the same data from the same parameters, refusal to run
twice). About 110 tests, under a minute.

## Configuration

All settings are environment variables (`infrastructure/.env-sample`, `server/src/config.rs`): HTTP port (`PORT`),
database (`CRM_DB_HOST`, `CRM_DB_PORT`, `CRM_DB_NAME`, `CRM_DB_APP_USER` / `_PASSWORD`, `CRM_DB_MIGRATOR_USER` /
`_PASSWORD`, `CRM_DB_ADMIN_USER` / `_PASSWORD` for `CRM_MODE=init`), pool size (`CRM_DB_POOL_SIZE`), connection
timeout (`CRM_DB_CONNECTION_TIMEOUT_MS`), session timeout (`CRM_SESSION_TIMEOUT`), secure cookies
(`CRM_COOKIE_SECURE`), lockout (`CRM_LOGIN_MAX_FAILED`, `CRM_LOGIN_LOCK_MINUTES`), first admin (`CRM_ADMIN_*`), dev
login (`CRM_DEV_LOGIN`), Swagger UI (`CRM_SWAGGER_UI`), web app directory (`CRM_STATIC_DIR`), public desk form limit
(`CRM_DESK_PUBLIC_REPORTS_PER_MINUTE`), attachments bucket (`CRM_S3_ENDPOINT`, `CRM_S3_ACCESS_KEY`,
`CRM_S3_SECRET_KEY`, `CRM_S3_BUCKET`, `CRM_S3_REGION`), seed (`CRM_SEED_*`), logs (`CRM_LOG`, `CRM_LOG_FORMAT=json`).

## Build

```bash
cd server && cargo build --release   # the binary target/release/crm
docker build -t crm-app:local .       # the image used by compose (web app built in a Node stage)
```

## License

[MIT](LICENSE)

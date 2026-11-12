# PostgreSQL run 1: mixed traffic, 200 VUs, the Rust API on PostgreSQL 18

Date: 2026-11-12, 05:08-05:21 UTC. Commit: `f9a10d6` (branch `chore/rust-pg-seed-perf-cleanup`).
The same k6 scenario (`perf/k6/crm-mixed.js`, unchanged), 200 VUs (2 min ramp, 10 min hold, 1 min down), the same
laptop and the same data as the SQL Server runs: `seed.run` with the defaults gives the same rows (the same
sha256 numbers as the T-SQL seed). All indexes of the schema in place (the PostgreSQL schema was written with the
indexes SQL Server got in run 2); a freshly seeded database, statistics cleared just before the run.

Result: **thresholds passed**; p95 **4.63 ms** and p99 **5.52 ms** against 8.5 / 10.4 ms of
[SQL Server run 2](../2026-11-08-run2-indexes/REPORT.md); no failed request or check; the database used about 0.3 of a
core on average.

## Result

| Threshold | Limit | SQL Server run 2 | PostgreSQL run 1 | |
|---|---|---:|---:|---|
| `http_req_duration` p95 | < 500 ms | 8.5 ms | **4.63 ms** | pass |
| `http_req_duration` p99 | < 1000 ms | 10.4 ms | **5.52 ms** | pass |
| `http_req_failed` | < 1% | 0.00% (1 of 263 307) | **0.00%** (0 of 264 711) | pass |
| median | - | - | 0.51 ms | |
| max | - | 163 ms (a login) | 167 ms (a project search) | |

Throughput: 264 711 requests, **339.3 req/s**, 211 714 iterations - as in run 2 the rate is set by the 200 VUs and
their think time, not by a bottleneck. Checks: 207 733 passed, 0 failed. No ERROR or WARN line in the application
log (`app-log.txt`).

### Latency per endpoint (ms)

| Endpoint | p95 SQL Server run 2 | p95 PostgreSQL | avg | p99 | max |
|---|---:|---:|---:|---:|---:|
| `GET /api/projects` (search) | 10.5 | **5.56** | 3.69 | 6.78 | 166.9 |
| `POST /api/auth/login` (bcrypt) | 57.7 | **51.21** | 45.02 | 52.91 | 63.3 |
| `GET /api/clients` (search) | 4.0 | **1.26** | 0.71 | 2.50 | 11.3 |
| `GET /api/clients` (next page) | 4.5 | **0.87** | 0.55 | 1.12 | 5.6 |
| `POST /api/projects/{id}/stage` | 3.4 | **1.79** | 1.34 | 2.67 | 18.0 |
| `POST /api/projects` | 2.6 | **1.58** | 1.27 | 2.18 | 25.5 |
| `POST /api/clients` | 2.3 | **1.34** | 1.04 | 1.66 | 18.5 |
| `POST /api/projects/{id}/comments` | 2.3 | **1.33** | 1.03 | 1.76 | 27.9 |
| `PUT /api/clients/{id}` | 2.1 | **1.63** | 1.20 | 2.35 | 17.6 |
| `GET /api/projects/{id}/comments` | 0.9 | **0.48** | 0.30 | 0.62 | 4.8 |
| `GET /api/projects/{id}/stages` | 0.7 | **0.50** | 0.29 | 0.65 | 5.7 |
| `GET /api/projects/{id}` | 0.8 | **0.47** | 0.30 | 0.58 | 4.1 |
| `GET /api/clients/{id}` | 0.7 | **0.40** | 0.25 | 0.53 | 4.5 |
| `GET /api/teams/{id}/stats` | 0.7 | **0.43** | 0.28 | 0.53 | 4.2 |

Login stays the slowest request: bcrypt (cost as before) is about 40 ms of CPU by design, once per VU.

### Resources (docker stats every 30 s, 26 samples)

| Container | CPU avg | CPU max | Memory | SQL Server run 2 |
|---|---:|---:|---:|---|
| postgres | **27%** | 39% | 2.3 GiB (2 GB of shared buffers) | mssql: CPU avg 58%, max 88%, 3.9 GiB |
| app (Rust) | **4%** | 13% | 36 MiB | app (JVM): CPU avg 9%, max 35%, about 480 MiB |

100% = one core.

## Database side

Per routine during the run (`procedure-stats.txt`, `pg_stat_user_functions`):

| Routine | Calls | avg ms | avg ms SQL Server run 2 | blocks per call (statement) | logical reads SQL Server run 2 |
|---|---:|---:|---:|---:|---:|
| `app.project_search` | 43 708 | 3.41 | 2.7 | 3 885 | 3 973 |
| `app.client_search` | 66 626 | 0.44 | 0.5 | 196 | 629 |
| `app.project_change_stage` | 7 285 | 0.47 | 2.1 | 87 | 117 |
| `app.project_create` | 5 182 | 0.34 | 1.6 | 77 | 85 |
| `app.project_get` | 43 944 | 0.10 | 0.1 | 22 | 17 |
| `app.team_project_stats` | 10 629 | 0.09 | 0.1 | 13 | 6 |

Cache hit ratio of the run on schema `crm`: 99.87%.

Single calls (`sql-reads.md`, `EXPLAIN (ANALYZE, BUFFERS)`, warm cache; 8 KB blocks like SQL Server pages):

| Call | Logical reads SQL Server run 2 | Shared buffers PostgreSQL | Elapsed SQL Server run 2 | Execution PostgreSQL |
|---|---:|---:|---:|---:|
| project search, all by due date (MANAGER) | 260 | 272 | 5 ms | 0.43 ms |
| project search, visible to a SALES user | 6 049 | 5 898 | 13 ms | 2.14 ms |
| project search, overdue, SALES user | 4 903 | 5 754 | 13 ms | 1.89 ms |
| client search, name prefix, SALES user | 115 | 27 | 2 ms | 0.09 ms |
| project search, overdue (MANAGER, partial index) | 263 | 278 | 6 ms | 0.44 ms |
| team statistics (stats table / indexed view) | 6 | 14 | 0 ms | 0.03 ms |
| OFFSET paging, page 200 000 of clients | - | 90 171 | - | 330 ms |
| keyset paging, the same page | - | 83 | - | 0.11 ms |

## What it shows

- The plans read about the same number of pages as the tuned SQL Server ones: the PostgreSQL schema carries the
  run-2 indexes from the start (`ix_projects_due`, the SALES search taking the first page of each source,
  `ix_clients_assigned_user_name`), plus a partial index for overdue projects and a stats table kept by the
  procedures instead of the indexed view.
- The remaining cost is the same as before: SALES project search reads about 4 000-6 000 blocks (the "projects of
  my clients" source probes `ix_projects_client` once per client). It is the slowest call (3.4 ms average) and still
  far below the thresholds, so nothing was changed (no second run needed: p95 is not worse than 8.5 ms).

## Not the same environment

The comparison is fair on data, scenario and hardware, not on everything else:

| | SQL Server run 2 | PostgreSQL run 1 |
|---|---|---|
| Edition limits | Express: at most 4 cores, buffer pool about 1.4 GB | none: 16 cores, `shared_buffers` 2 GB + OS cache |
| API | Kotlin, Spring Boot 4 on the JVM, JDBC (HikariCP) | Rust, axum, tokio-postgres (deadpool, 30 connections) |
| Data | seeded + about 24 000 rows written by run 1 | freshly seeded |
| Database size | 7.6 GB files | 5.4 GB |

The database is not the limit in either run (CPU well under its cap), so the latency gain comes mostly from shorter
single calls and a lighter API, not from more cores; but the numbers are not a benchmark of SQL Server against
PostgreSQL.

## Files

`summary.json` / `summary.txt` (k6), `k6-console.txt`, `docker-stats.txt`, `procedure-stats.txt` (`pg_stat_user_functions`
and `pg_stat_statements` of the run), `sql-reads.md`, `app-log.txt`.

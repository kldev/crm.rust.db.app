# Run 2: mixed traffic, 200 VUs, after the read indexes

Date: 2026-11-08, 12:27-12:40 UTC. Commit: `40a513f` (schema V4: `ix_projects_due`, `ix_clients_assigned_user_name`,
SALES project search rewritten to take the first page of each source).
Same environment, data and k6 scenario as [run 1](../2026-11-08-run1-baseline/REPORT.md); the database kept the
rows written by run 1 (about 24 000 extra clients, projects and comments).

Result: **thresholds passed**; p95 6.6x and p99 7.8x lower than run 1, worst request 163 ms instead of 6.76 s,
database CPU 3.4x lower.

## Result

| Threshold | Limit | Run 1 | Run 2 | |
|---|---|---:|---:|---|
| `http_req_duration` p95 | < 500 ms | 56.1 ms | **8.5 ms** | pass |
| `http_req_duration` p99 | < 1000 ms | 81.2 ms | **10.4 ms** | pass |
| `http_req_failed` | < 1% | 0.00% | **0.00%** (1 of 263 307) | pass |
| max | - | 6 760 ms | **163 ms** (a login) | |

Throughput: 263 307 requests, **337.5 req/s**, 211 121 iterations (the rate is set by the 200 VUs and their think
time, not by a bottleneck). Checks: 206 370 passed, 1 failed - one `POST /api/projects/{id}/stage` out of 6 757
did not return 200. There was no server error in the application log; k6 did not record the status. Most likely
a 409 from optimistic concurrency (two VUs moved the same project from the same `rowVersion`), which is the
intended behaviour, but this is not confirmed.

### Latency per endpoint (ms)

| Endpoint | p95 run 1 | p95 run 2 | avg run 2 | p99 run 2 | max run 2 |
|---|---:|---:|---:|---:|---:|
| `GET /api/projects` (search) | 85.2 | **10.5** | 7.3 | 12.3 | 74.0 |
| `POST /api/auth/login` (bcrypt) | 56.7 | 57.7 | 47.1 | 71.1 | 163.4 |
| `GET /api/clients` (search) | 12.0 | **4.0** | 2.6 | 6.4 | 46.2 |
| `GET /api/clients` (next page) | 7.9 | 4.5 | 2.8 | 6.3 | 35.1 |
| `POST /api/projects/{id}/stage` | 7.4 | 3.4 | 2.5 | 5.3 | 63.1 |
| `POST /api/projects` | 6.9 | 2.6 | 2.1 | 4.6 | 41.1 |
| `POST /api/clients` | 6.3 | 2.3 | 1.9 | 4.8 | 47.3 |
| `POST /api/projects/{id}/comments` | 6.0 | 2.3 | 1.9 | 4.2 | 39.7 |
| `PUT /api/clients/{id}` | 4.4 | 2.1 | 1.8 | 3.4 | 45.0 |
| `GET /api/projects/{id}/comments` | 2.6 | 0.9 | 0.6 | 1.7 | 38.0 |
| `GET /api/projects/{id}/stages` | 1.3 | 0.7 | 0.5 | 1.1 | 31.4 |
| `GET /api/projects/{id}` | 1.4 | 0.8 | 0.5 | 1.3 | 29.6 |
| `GET /api/clients/{id}` | 1.3 | 0.7 | 0.5 | 1.2 | 31.0 |
| `GET /api/teams/{id}/stats` (indexed view) | 1.3 | 0.7 | 0.5 | 1.2 | 29.3 |

Login is now the slowest request: bcrypt costs about 40 ms of CPU by design and happens once per VU.
Writes got faster too: the expensive searches no longer compete with them for the 4 cores of Express.

### Resources (docker stats every 30 s)

| Container | CPU avg run 1 | CPU avg run 2 | CPU max run 2 | Memory |
|---|---:|---:|---:|---:|
| mssql | 198% | **58%** | 88% | 3.9 GiB |
| app | 9% | 9% | 35% | about 480 MiB |

## Database side

Per procedure during the run (`procedure-stats.txt`):

| Procedure | Calls | avg ms run 1 | avg ms run 2 | max ms run 2 | avg logical reads run 1 | run 2 |
|---|---:|---:|---:|---:|---:|---:|
| `app.project_search` | 43 717 | 30.6 | **2.7** | 37.5 | 23 139 | **3 973** |
| `app.client_search` | 66 278 | 3.3 | **0.5** | 33.8 | 3 695 | **629** |
| `app.project_change_stage` | 6 757 | 3.4 | 2.1 | 40.1 | 109 | 117 |
| `app.project_create` | 5 105 | 2.5 | 1.6 | 30.9 | 81 | 85 |
| `app.project_get` | 31 692 | 0.2 | 0.1 | 1.2 | 17 | 17 |
| `app.team_project_stats` | 10 410 | 0.1 | 0.1 | 0.4 | 6 | 6 |

Single calls with `SET STATISTICS IO` (`sql-reads.md`, warm cache):

| Call | Reads run 1 | Reads run 2 | Elapsed run 1 | Elapsed run 2 |
|---|---:|---:|---:|---:|
| project search, all by due date (MANAGER) | 102 267 | **260** | 8 628 ms | **5 ms** |
| project search, visible to a SALES user | 21 671 | **6 049** | 48 ms | **13 ms** |
| project search, overdue, SALES user | 21 671 | **4 903** | 48 ms | **13 ms** |
| client search, name prefix, SALES user | 4 541 | **115** | 5 ms | **2 ms** |
| project search, overdue (MANAGER, filtered index) | 469 | 263 | 7 ms | 6 ms |
| team statistics (indexed view) | 6 | 6 | 0 ms | 0 ms |

## What changed

1. `ix_projects_due (due_date, id)` - the unfiltered project list is a range seek + TOP like every other list.
2. SALES project search: instead of collecting every visible project and sorting them, each source (own
   projects, projects of own teams, projects of own clients) gives only its first `page_size + 1` rows after the
   keyset from an index ending in `(due_date, id)`; the page is taken from their union. All filters are applied
   inside the sources, so the page stays correct.
3. `ix_clients_assigned_user_name (assigned_user_id, name, id)` - SALES name search is a seek.

## Remaining cost

SALES project search still reads about 5 000-6 000 pages: the "projects of my clients" source probes
`ix_projects_client` once per client of the user (about 1 100 clients x 3 pages). Going lower would need
denormalisation (for example the client owner copied to `crm.projects`), which the current numbers (13 ms
single call, p95 10.5 ms under load) do not justify.

## Files

`summary.json` / `summary.txt` (k6), `k6-console.txt`, `docker-stats.txt`, `procedure-stats.txt`, `sql-reads.md`.

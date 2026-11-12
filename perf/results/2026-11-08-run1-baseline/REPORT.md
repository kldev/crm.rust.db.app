# Run 1: mixed traffic, 200 VUs, baseline

Date: 2026-11-08, 12:07-12:20 UTC. Commit: `622f5e3` (schema V3, before the read indexes of V4).
Result: **thresholds passed**, but project search had an 8.6 s worst case. Fixed in run 2.

## Environment

| | |
|---|---|
| Host | laptop, 16 cores, 30 GB RAM, Linux, Docker 29.9, everything on one machine (k6 included) |
| Database | SQL Server 2025 **Express** RTM-CU10 (17.0.5035.5) in Docker, `MSSQL_PID=Express` |
| Express limits | 50 GB per database, about 1.4 GB buffer pool, the lesser of 1 socket or 4 cores |
| Application | Spring Boot 4.1, JDK 25, virtual threads, Hikari pool 30, one instance |
| Data | 5 000 000 clients, 5 000 000 projects (+5 000 000 stage history rows), 2 000 000 comments, 5 000 users, 900 teams; database 7.6 GB |
| Load | k6 2.3, `perf/k6/crm-mixed.js`: 0 -> 200 VUs in 2 min, 200 VUs for 10 min, down in 1 min; think time 0.3-1.0 s |
| Users | 180 VUs as `perf.sales.NNNN`, 20 as `perf.manager.NNNN`; each logs in once |

Iteration mix: client search 25%, project search 20%, client details 10%, project details + stage history 10%,
comments list 8%, add comment 5%, stage change 5%, client update 5%, team statistics 5%, client create 4%,
project create 3%.

## Result

| Threshold | Limit | Measured | |
|---|---|---|---|
| `http_req_duration` p95 | < 500 ms | **56.1 ms** | pass |
| `http_req_duration` p99 | < 1000 ms | **81.2 ms** | pass |
| `http_req_failed` | < 1% | **0.00%** (0 of 260 563) | pass |

Throughput: 260 563 requests, **334 req/s**, 208 318 iterations, all 204 172 checks passed.

### Latency per endpoint (ms)

| Endpoint | avg | p50 | p95 | p99 | max |
|---|---:|---:|---:|---:|---:|
| `GET /api/projects` (search) | 45.3 | 47.1 | **85.2** | 110.9 | **6 760** |
| `POST /api/auth/login` (bcrypt) | 45.5 | 43.2 | 56.7 | 68.8 | 75.1 |
| `GET /api/clients` (search) | 6.3 | 5.3 | 12.0 | 24.9 | 253.5 |
| `GET /api/clients` (next page) | 4.4 | 4.2 | 7.9 | 13.4 | 93.2 |
| `POST /api/projects/{id}/stage` | 3.9 | 3.1 | 7.4 | 21.4 | 83.8 |
| `POST /api/projects` | 3.2 | 2.4 | 6.9 | 18.7 | 78.1 |
| `POST /api/clients` | 2.9 | 2.2 | 6.3 | 16.3 | 49.9 |
| `POST /api/projects/{id}/comments` | 2.8 | 2.1 | 6.0 | 17.1 | 48.8 |
| `PUT /api/clients/{id}` | 2.5 | 2.0 | 4.4 | 13.4 | 49.1 |
| `GET /api/projects/{id}/comments` | 1.0 | 0.7 | 2.6 | 9.0 | 40.0 |
| `GET /api/projects/{id}/stages` | 0.8 | 0.6 | 1.3 | 8.2 | 31.7 |
| `GET /api/projects/{id}` | 0.7 | 0.5 | 1.4 | 4.9 | 47.2 |
| `GET /api/clients/{id}` | 0.6 | 0.5 | 1.3 | 4.5 | 40.9 |
| `GET /api/teams/{id}/stats` (indexed view) | 0.6 | 0.5 | 1.3 | 4.3 | 42.0 |

### Resources (docker stats every 30 s)

| Container | CPU avg | CPU max | Memory |
|---|---:|---:|---:|
| mssql | 198% (2 cores) | 313% | 3.9 GiB |
| app | 9% | 25% | 483 MiB |

The application is idle compared with the database: the whole cost is in SQL Server, which stays under the
Express cap of 4 cores.

## Database side

Per procedure during the run (`procedure-stats.txt`, `sys.dm_exec_procedure_stats`):

| Procedure | Calls | avg ms | max ms | avg logical reads | max logical reads |
|---|---:|---:|---:|---:|---:|
| `app.project_search` | 42 758 | **30.6** | **6 760** | **23 139** | **102 084** |
| `app.client_search` | 65 970 | 3.3 | 248 | 3 695 | 12 194 |
| `app.project_change_stage` | 7 281 | 3.4 | 80 | 109 | 145 |
| `app.client_create` | 8 344 | 2.3 | 45 | 31 | 68 |
| `app.project_get` | 31 280 | 0.2 | 13 | 17 | 18 |
| `app.team_project_stats` | 10 531 | 0.1 | 8 | 6 | 8 |

Single calls with `SET STATISTICS IO` (`sql-reads.md`, warm cache):

| Call | Logical reads | Elapsed |
|---|---:|---:|
| project search, all projects by due date (MANAGER) | **102 267** | **8 628 ms** |
| project search, visible to a SALES user | **21 671** | 48 ms |
| client search, name prefix, SALES user | 4 541 | 5 ms |
| client search, newest first (MANAGER) | 53 | 2 ms |
| team statistics from the indexed view | 6 | 0 ms |
| same statistics counted from `crm.projects` | 16 867 | 6 ms |
| clients page 200 000 with `OFFSET` | 96 940 | 471 ms |
| the same page with keyset (`client_search`) | 61 | 3 ms |

## Findings

1. **No index for "all projects by due date".** Every other project list has an index ending in `(due_date, id)`;
   the unfiltered one (MANAGER without filters, rare in this mix) scanned the clustered
   index and sorted: 102 000 pages, up to 8.6 s. This is the 6.76 s maximum of the run.
2. **SALES project search materialised every visible project** (own, team, client projects: about 6 700 ids)
   before sorting and taking one page: 21 700 pages per call. It dominated the average of `project_search`.
3. SALES name search filtered the whole prefix range by owner (4 500 pages); small, but avoidable.
4. What already works: keyset paging (61 pages for page 200 000, `OFFSET` needs 96 940), the indexed view
   (6 pages vs 16 867), point reads and writes in 1-4 ms.

Fixes (`V4__read_indexes.sql`, `R__app_project_search.sql`) and their effect: [run 2](../2026-11-08-run2-indexes/REPORT.md).

## Files

`summary.json` / `summary.txt` (k6), `k6-console.txt`, `docker-stats.txt`, `procedure-stats.txt`, `sql-reads.md`.

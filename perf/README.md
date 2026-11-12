# Performance

Load tests with k6 against the seeded data set (one seed, `CRM_MODE=seed`), everything on one laptop (16 cores,
30 GB RAM, Docker). Thresholds: p95 < 500 ms, p99 < 1 s, errors < 1%. The project ran first on Kotlin + SQL Server
2025 **Express** and now runs on Rust + **PostgreSQL 18**; the scenario, the data (the PostgreSQL seed produces the
same rows as the T-SQL one) and the hardware are the same.

| Run | Stack | Change | VUs | Requests | p95 | p99 | Errors | DB CPU avg | Report |
|---|---|---|---:|---:|---:|---:|---:|---:|---|
| 1 | Kotlin + SQL Server Express | baseline (schema V3) | 200 | 260 563 (334/s) | 56.1 ms | 81.2 ms | 0.00% | 198% | [REPORT](results/2026-11-08-run1-baseline/REPORT.md) |
| 2 | Kotlin + SQL Server Express | read indexes + SALES project search (V4) | 200 | 263 307 (338/s) | 8.5 ms | 10.4 ms | 0.00% | 58% | [REPORT](results/2026-11-08-run2-indexes/REPORT.md) |
| PG 1 | Rust + PostgreSQL 18.6 | the schema with the run-2 indexes | 200 | 264 711 (339/s) | **4.63 ms** | **5.52 ms** | 0.00% | **27%** | [REPORT](results/2026-11-12-pg-run1/REPORT.md) |

100% CPU = one core. The PostgreSQL schema was written with the indexes SQL Server got in run 2, so there is no
PostgreSQL run without them; p95 was already below run 2, so no index was added after the run.

Each run directory holds `summary.json` and `summary.txt` (k6), `k6-console.txt`, `docker-stats.txt`,
`procedure-stats.txt` and `sql-reads.md` (pages per call) and `REPORT.md`. For SQL Server the statistics come from
`sys.dm_exec_procedure_stats` and `SET STATISTICS IO` (those scripts are in the git history before the Kotlin code
was removed); for PostgreSQL from `pg_stat_user_functions`, `pg_stat_statements` and `EXPLAIN (ANALYZE, BUFFERS)`.

## Run it (PostgreSQL)

```bash
cd infrastructure
./start.sh --clean --seed                         # data set: about 6 min
../perf/sql/measure-reads.sh "$PWD/../perf/results/2026-11-12-my-run"   # shared buffers per function (sql-reads.md)
../perf/sql/run-stats.sh reset                    # clear pg_stat_statements and the routine statistics
./start.sh --k6 crm-mixed.js 2026-11-12-my-run    # 200 VUs, 13 min; K6_VUS / K6_RAMP / K6_HOLD / K6_RAMP_DOWN override
../perf/sql/run-stats.sh "$PWD/../perf/results/2026-11-12-my-run"       # procedure-stats.txt of the run
```

`docker stats` during the run (`docker-stats.txt`) is sampled by hand, e.g. every 30 s:
`while sleep 30; do date -u +%T; docker stats --no-stream --format '{{.Name}} {{.CPUPerc}} {{.MemUsage}}' crm-postgres-1 crm-app-1; done`.

`perf/k6/crm-mixed.js`: each VU logs in once (90% `perf.sales.NNNN`, 10% `perf.manager.NNNN`) and repeats a
weighted action: searches and details dominate, about a quarter of iterations write (create/update client, add
comment, change stage, create project).

## PostgreSQL settings

Stated in `infrastructure/docker-compose.yml` (`command: -c ...`), each with its reason:

| Setting | Value | Why |
|---|---|---|
| `shared_buffers` | 2 GB | the hot part of the data set (indexes of clients and projects) fits; Express had a 1.4 GB buffer pool |
| `effective_cache_size` | 8 GB | the OS cache of the laptop holds the rest |
| `work_mem` | 16 MB | page sorts are tiny; safe for 30 API connections |
| `maintenance_work_mem` | 1 GB | index builds after the seed's bulk load |
| `max_wal_size` | 8 GB | the seed writes a few GB in one transaction |
| `random_page_cost` | 1.1 | local NVMe |
| `max_connections` | 100 | API pool 30 + migrate, seed, psql |
| `shared_preload_libraries`, `pg_stat_statements.track`, `track_functions`, `track_io_timing` | `pg_stat_statements`, `all`, `pl`, `on` | statistics for `perf/sql`, also of the statements inside routines |

## Comparing with SQL Server, honestly

| | SQL Server (runs 1-2) | PostgreSQL (PG 1) |
|---|---|---|
| Edition limits | Express: at most 4 cores, buffer pool about 1.4 GB | none: 16 cores available, `shared_buffers` 2 GB + OS cache |
| API | Kotlin, Spring Boot 4 on the JVM (about 480 MiB) | Rust, axum, tokio-postgres (about 36 MiB) |
| Data | seeded; run 2 also had about 24 000 rows written by run 1 | freshly seeded |
| Database size after the seed | 7.6 GB files (data 2.4 GB, indexes 3.5 GB) | 5.4 GB (tables 1.9 GB, indexes 3.5 GB) |
| Seed time | 798 s | 375 s |

In neither run the database was the limit (CPU far below its cap), so the edition limits did not decide the result;
the gain comes from shorter single calls (the same plans read about the same number of pages, but run faster) and a
lighter API. It is a comparison of two versions of this application, not a benchmark of SQL Server against
PostgreSQL.

Where the time goes now: SALES project search (`app.project_search`, 3.4 ms average, about 4 000-6 000 blocks per
call - one probe of `ix_projects_client` per client of the user) and the login (bcrypt, about 40 ms of CPU by
design). Lowering the first would need denormalisation (the client's owner copied to `crm.projects`), which these
numbers do not justify.

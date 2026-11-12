/*
  What the database did during a k6 run: per PL/pgSQL routine of schema app (pg_stat_user_functions, track_functions
  = 'pl') and the heaviest statements (pg_stat_statements, track = all, so the statements inside the routines too).
  perf/sql/run-stats.sh reset   - before the run (clears both)
  perf/sql/run-stats.sh <dir>   - after the run, writes <dir>/procedure-stats.txt
*/
\pset footer off
\echo '== Routines of schema app (calls, total and own time; own time excludes nested routines)'
SELECT f.schemaname || '.' || f.funcname                  AS routine,
       f.calls,
       round((f.total_time / nullif(f.calls, 0))::numeric, 3) AS avg_ms,
       round((f.self_time / nullif(f.calls, 0))::numeric, 3)  AS avg_self_ms,
       round(f.total_time::numeric)                          AS total_ms
FROM pg_stat_user_functions AS f
WHERE f.schemaname = 'app'
ORDER BY f.total_time DESC;

\echo
\echo '== Top 25 statements by total time (inside routines included; blocks per call = shared hit + read)'
SELECT left(regexp_replace(s.query, '\s+', ' ', 'g'), 110)                    AS statement,
       s.calls,
       round(s.mean_exec_time::numeric, 3)                                     AS avg_ms,
       round(s.max_exec_time::numeric, 2)                                      AS max_ms,
       round(((s.shared_blks_hit + s.shared_blks_read) / nullif(s.calls, 0))::numeric, 1) AS blocks_per_call,
       s.shared_blks_read                                                      AS blocks_read,
       s.toplevel
FROM pg_stat_statements AS s
WHERE s.dbid = (SELECT oid FROM pg_database WHERE datname = current_database())
ORDER BY s.total_exec_time DESC
LIMIT 25;

\echo
\echo '== Cache hit ratio of the run (tables and indexes of schema crm)'
SELECT round(100.0 * sum(heap_blks_hit + coalesce(idx_blks_hit, 0))
           / nullif(sum(heap_blks_hit + heap_blks_read + coalesce(idx_blks_hit, 0) + coalesce(idx_blks_read, 0)), 0), 2)
           AS hit_percent
FROM pg_statio_user_tables
WHERE schemaname = 'crm';

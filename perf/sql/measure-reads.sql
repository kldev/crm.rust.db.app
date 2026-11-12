/*
  Shared buffers (8 KB blocks, hit + read) and time of each read function on the seeded database, the PostgreSQL
  counterpart of the SQL Server "logical reads" (SET STATISTICS IO). Run after ./start.sh --seed:
      perf/sql/measure-reads.sh <results dir>
  Each call is EXPLAIN (ANALYZE, BUFFERS)'d three times and the last (warm cache, cached plans) is reported: the
  Function Scan node counts the buffers of every statement the function runs. Two comparisons show why the
  functions look the way they do: OFFSET paging vs keyset, team statistics counted vs read from the stats table.
*/
\set QUIET on
\pset format unaligned
\pset tuples_only on

CREATE FUNCTION pg_temp.measure(p_label text, p_sql text)
    RETURNS text
    LANGUAGE plpgsql
AS
$$
DECLARE
    v_plan jsonb;
    v_node jsonb;
BEGIN
    FOR i IN 1..3
        LOOP
            EXECUTE 'EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) ' || p_sql INTO v_plan;
        END LOOP;
    v_node := v_plan -> 0 -> 'Plan';
    RETURN format('| %s | %s | %s | %s | %s |', p_label,
                  (v_node ->> 'Shared Hit Blocks')::bigint + (v_node ->> 'Shared Read Blocks')::bigint,
                  v_node ->> 'Shared Read Blocks',
                  round((v_plan -> 0 ->> 'Planning Time')::numeric, 2),
                  round((v_plan -> 0 ->> 'Execution Time')::numeric, 2));
END;
$$;

SELECT id AS sales FROM crm.users WHERE login = 'perf.sales.0001' \gset
SELECT id AS manager FROM crm.users WHERE login = 'perf.manager.0001' \gset
SELECT max(id) AS client FROM crm.clients WHERE assigned_user_id = :sales \gset
SELECT max(id) AS project FROM crm.projects WHERE owner_user_id = :sales \gset
SELECT team_id AS team FROM crm.team_members WHERE user_id = :sales \gset
SELECT nip FROM crm.clients WHERE id = (SELECT min(id) + 2499999 FROM crm.clients) \gset
SELECT max(id) - 4000000 + 1 AS after_id FROM crm.clients \gset

SELECT '| Call | Shared buffers (8 KB blocks) | of which read from disk | Planning ms | Execution ms |';
SELECT '|---|---:|---:|---:|---:|';
SELECT pg_temp.measure('client_search: newest first (MANAGER, 5M rows)',
    format('SELECT * FROM app.client_search(%s, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 20)', :manager));
SELECT pg_temp.measure('client_search: name prefix Zielony (MANAGER)',
    format('SELECT * FROM app.client_search(%s, %L, NULL, NULL, NULL, NULL, NULL, NULL, 20)', :manager, 'Zielony'));
SELECT pg_temp.measure('client_search: name prefix Zielony (SALES, own clients)',
    format('SELECT * FROM app.client_search(%s, %L, NULL, NULL, NULL, NULL, NULL, NULL, 20)', :sales, 'Zielony'));
SELECT pg_temp.measure('client_search: city + status (MANAGER)',
    format('SELECT * FROM app.client_search(%s, NULL, NULL, NULL, %L, %L, NULL, NULL, 20)', :manager, 'Gdańsk', 'ACTIVE'));
SELECT pg_temp.measure('client_search: exact NIP (MANAGER)',
    format('SELECT * FROM app.client_search(%s, NULL, %L, NULL, NULL, NULL, NULL, NULL, 20)', :manager, :'nip'));
SELECT pg_temp.measure('client_get', format('SELECT * FROM app.client_get(%s, %s)', :sales, :client));

SELECT pg_temp.measure('project_search: all by due date (MANAGER, 5M rows)',
    format('SELECT * FROM app.project_search(%s, NULL, NULL, NULL, NULL, false, NULL, NULL, NULL, NULL, 20)', :manager));
SELECT pg_temp.measure('project_search: overdue (MANAGER, partial index)',
    format('SELECT * FROM app.project_search(%s, NULL, NULL, NULL, NULL, true, NULL, NULL, NULL, NULL, 20)', :manager));
SELECT pg_temp.measure('project_search: stage ACTIVE (MANAGER)',
    format('SELECT * FROM app.project_search(%s, %L, NULL, NULL, NULL, false, NULL, NULL, NULL, NULL, 20)', :manager, 'ACTIVE'));
SELECT pg_temp.measure('project_search: team (MANAGER)',
    format('SELECT * FROM app.project_search(%s, NULL, %s, NULL, NULL, false, NULL, NULL, NULL, NULL, 20)', :manager, :team));
SELECT pg_temp.measure('project_search: visible to SALES (own + team + clients)',
    format('SELECT * FROM app.project_search(%s, NULL, NULL, NULL, NULL, false, NULL, NULL, NULL, NULL, 20)', :sales));
SELECT pg_temp.measure('project_search: overdue visible to SALES',
    format('SELECT * FROM app.project_search(%s, NULL, NULL, NULL, NULL, true, NULL, NULL, NULL, NULL, 20)', :sales));
SELECT pg_temp.measure('project_get', format('SELECT * FROM app.project_get(%s, %s)', :sales, :project));
SELECT pg_temp.measure('project_stage_history_list',
    format('SELECT * FROM app.project_stage_history_list(%s, %s)', :sales, :project));
SELECT pg_temp.measure('project_comment_list',
    format('SELECT * FROM app.project_comment_list(%s, %s, NULL, 20)', :sales, :project));
SELECT pg_temp.measure('team_project_stats (crm.team_project_stats)',
    format('SELECT * FROM app.team_project_stats(%s, %s)', :manager, :team));

SELECT pg_temp.measure('COMPARISON team stats counted from crm.projects',
    format('SELECT stage, count(*), sum(budget) FROM crm.projects WHERE team_id = %s GROUP BY stage', :team));
SELECT pg_temp.measure('COMPARISON OFFSET paging, page 200 000 of clients',
    'SELECT id, name FROM crm.clients ORDER BY id DESC OFFSET 4000000 LIMIT 20');
SELECT pg_temp.measure('COMPARISON keyset paging, the same page',
    format('SELECT * FROM app.client_search(%s, NULL, NULL, NULL, NULL, NULL, %s, NULL, 20)', :manager, :after_id));

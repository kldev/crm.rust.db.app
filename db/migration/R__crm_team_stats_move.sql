/*
  Internal helper (ADR-0015): moves one project in crm.team_project_stats from (old team, old stage, old budget)
  to (new team, new stage, new budget). NULL old team = a new project. Called by every procedure that writes
  crm.projects, in the same transaction. The two rows are always updated in (team_id, stage) order, so two
  concurrent moves between the same rows cannot deadlock.
*/
CREATE OR REPLACE FUNCTION crm.team_stats_move(
    p_old_team_id int,
    p_old_stage   text,
    p_old_budget  numeric,
    p_new_team_id int,
    p_new_stage   text,
    p_new_budget  numeric
)
    RETURNS void
    LANGUAGE plpgsql
AS
$$
DECLARE
    r record;
BEGIN
    IF (p_old_team_id, p_old_stage, p_old_budget) IS NOT DISTINCT FROM (p_new_team_id, p_new_stage, p_new_budget) THEN
        RETURN;
    END IF;

    FOR r IN
        SELECT d.team_id, d.stage, sum(d.n) AS n, sum(d.budget) AS budget
        FROM (VALUES (p_old_team_id, p_old_stage, -1, -p_old_budget),
                     (p_new_team_id, p_new_stage, 1, p_new_budget)) AS d (team_id, stage, n, budget)
        WHERE d.team_id IS NOT NULL
        GROUP BY d.team_id, d.stage
        ORDER BY d.team_id, d.stage
        LOOP
            -- The row first (CHECK is evaluated on a proposed INSERT row even when it conflicts), then the delta.
            INSERT INTO crm.team_project_stats (team_id, stage)
            VALUES (r.team_id, r.stage)
            ON CONFLICT (team_id, stage) DO NOTHING;

            UPDATE crm.team_project_stats AS s
            SET project_count = s.project_count + r.n,
                budget_total  = s.budget_total + r.budget
            WHERE s.team_id = r.team_id
              AND s.stage = r.stage;
        END LOOP;
END;
$$;

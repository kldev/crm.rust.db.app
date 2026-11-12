-- Projects and budget of a team per stage, from crm.team_project_stats: at most 6 rows read, however many projects.
CREATE OR REPLACE FUNCTION app.team_project_stats(p_actor_id int, p_team_id int)
    RETURNS TABLE
            (
                stage         text,
                project_count bigint,
                budget_total  numeric
            )
    LANGUAGE plpgsql
    STABLE
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
BEGIN
    PERFORM crm.actor_require(p_actor_id);
    IF NOT EXISTS (SELECT 1 FROM crm.teams AS t WHERE t.id = p_team_id) THEN
        RAISE EXCEPTION 'Team not found.' USING ERRCODE = 'U1404';
    END IF;

    RETURN QUERY
        SELECT s.stage, s.project_count, s.budget_total
        FROM crm.team_project_stats AS s
        WHERE s.team_id = p_team_id
          AND s.project_count > 0
        ORDER BY s.stage;
END;
$$;

/*
  Number of projects matching the filters of app.project_search, with the same visibility (SALES: own projects,
  projects of own teams, projects of own clients, each counted once). Not part of the list: a count reads every
  matching row, so the list stays keyset-only and the UI calls this only when the user asks for the total.
*/
CREATE OR REPLACE FUNCTION app.project_count(
    p_actor_id      int,
    p_stage         text,
    p_team_id       int,
    p_client_id     bigint,
    p_owner_user_id int,
    p_overdue_only  boolean,
    p_due_from      date,
    p_due_to        date
)
    RETURNS TABLE (total bigint)
    LANGUAGE plpgsql
    STABLE
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
    SET plan_cache_mode = force_custom_plan
AS
$$
DECLARE
    v_actor_role text := crm.actor_require(p_actor_id);
    v_today      date := (now() AT TIME ZONE 'UTC')::date;
BEGIN
    IF p_stage IS NOT NULL AND p_stage NOT IN ('NEW', 'ACCEPTED', 'ACTIVE', 'ON_HOLD', 'CLOSED', 'DECLINED') THEN
        RAISE EXCEPTION 'Stage must be NEW, ACCEPTED, ACTIVE, ON_HOLD, CLOSED or DECLINED.' USING ERRCODE = 'U1031';
    END IF;

    p_overdue_only := coalesce(p_overdue_only, false);
    IF p_overdue_only AND (p_due_to IS NULL OR p_due_to >= v_today) THEN
        p_due_to := v_today - 1;
    END IF;

    IF v_actor_role = 'SALES' THEN
        -- UNION (not UNION ALL): a project visible through two sources is counted once.
        RETURN QUERY
            SELECT count(*)
            FROM (SELECT p.id
                  FROM crm.projects AS p
                  WHERE p.owner_user_id = p_actor_id
                  UNION
                  SELECT p.id
                  FROM crm.team_members AS tm
                  JOIN crm.projects AS p ON p.team_id = tm.team_id
                  WHERE tm.user_id = p_actor_id
                  UNION
                  SELECT p.id
                  FROM crm.clients AS c
                  JOIN crm.projects AS p ON p.client_id = c.id
                  WHERE c.assigned_user_id = p_actor_id) AS visible
            JOIN crm.projects AS p ON p.id = visible.id
            WHERE (p_stage IS NULL OR p.stage = p_stage)
              AND (p_team_id IS NULL OR p.team_id = p_team_id)
              AND (p_client_id IS NULL OR p.client_id = p_client_id)
              AND (p_owner_user_id IS NULL OR p.owner_user_id = p_owner_user_id)
              AND (NOT p_overdue_only OR p.stage IN ('NEW', 'ACCEPTED', 'ACTIVE', 'ON_HOLD'))
              AND (p_due_from IS NULL OR p.due_date >= p_due_from)
              AND (p_due_to IS NULL OR p.due_date <= p_due_to);
        RETURN;
    END IF;

    RETURN QUERY
        SELECT count(*)
        FROM crm.projects AS p
        WHERE (p_stage IS NULL OR p.stage = p_stage)
          AND (p_team_id IS NULL OR p.team_id = p_team_id)
          AND (p_client_id IS NULL OR p.client_id = p_client_id)
          AND (p_owner_user_id IS NULL OR p.owner_user_id = p_owner_user_id)
          AND (NOT p_overdue_only OR p.stage IN ('NEW', 'ACCEPTED', 'ACTIVE', 'ON_HOLD'))
          AND (p_due_from IS NULL OR p.due_date >= p_due_from)
          AND (p_due_to IS NULL OR p.due_date <= p_due_to);
END;
$$;

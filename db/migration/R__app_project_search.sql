/*
  Projects page from crm.v_projects, ordered by (due_date, id), keyset pagination: next page after
  (p_after_due_date, p_after_id). Returns p_page_size + 1 rows so the caller knows if there is a next page.

  Filters: stage, team, client, owner, due date range, overdue only. Each filter has an index that starts with it
  and ends with (due_date, id), so a page is a range scan plus LIMIT, whatever the size of crm.projects.
  plan_cache_mode = force_custom_plan: every call is planned for the filters actually passed.

  SALES: visible projects come from three indexed sources (own projects, projects of own teams, projects of own
  clients); each source contributes only its first page, so the cost does not grow with the number of
  visible projects.
*/
CREATE OR REPLACE FUNCTION app.project_search(
    p_actor_id       int,
    p_stage          text,
    p_team_id        int,
    p_client_id      bigint,
    p_owner_user_id  int,
    p_overdue_only   boolean,
    p_due_from       date,
    p_due_to         date,
    p_after_due_date date,
    p_after_id       bigint,
    p_page_size      int
)
    RETURNS TABLE
            (
                id                 bigint,
                name               text,
                stage              text,
                start_date         date,
                due_date           date,
                budget             numeric,
                is_overdue         boolean,
                stage_changed_at   timestamptz,
                client_id          bigint,
                client_name        text,
                client_city        text,
                team_id            int,
                team_name          text,
                owner_user_id      int,
                owner_login        text,
                owner_display_name text,
                updated_at         timestamptz,
                row_version        text
            )
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
    v_n          int;
BEGIN
    IF p_page_size IS NULL OR p_page_size NOT BETWEEN 1 AND 200 THEN
        RAISE EXCEPTION 'Page size must be between 1 and 200.' USING ERRCODE = 'U1014';
    END IF;
    IF p_stage IS NOT NULL AND p_stage NOT IN ('NEW', 'ACCEPTED', 'ACTIVE', 'ON_HOLD', 'CLOSED', 'DECLINED') THEN
        RAISE EXCEPTION 'Stage must be NEW, ACCEPTED, ACTIVE, ON_HOLD, CLOSED or DECLINED.' USING ERRCODE = 'U1031';
    END IF;

    v_n := p_page_size + 1;
    p_overdue_only := coalesce(p_overdue_only, false);
    -- Overdue projects: due before today.
    IF p_overdue_only AND (p_due_to IS NULL OR p_due_to >= v_today) THEN
        p_due_to := v_today - 1;
    END IF;
    -- Lower bound for the keyset: lets every branch start the index range at the next page.
    IF p_after_due_date IS NOT NULL AND (p_due_from IS NULL OR p_due_from < p_after_due_date) THEN
        p_due_from := p_after_due_date;
    END IF;

    IF v_actor_role = 'SALES' THEN
        /*
          Visible = own projects + projects of own teams + projects of own clients. Each source has an index that
          ends with (due_date, id), so each one returns only its first p_page_size + 1 rows after the keyset, with
          every filter applied; the page is the first p_page_size + 1 of their union. Every filter must be inside
          the branches: a filter applied after LIMIT would drop rows and break the page.
        */
        RETURN QUERY
            WITH candidates AS (
                (SELECT p.id, p.due_date
                 FROM crm.projects AS p
                 WHERE p.owner_user_id = p_actor_id
                   AND (p_stage IS NULL OR p.stage = p_stage)
                   AND (p_team_id IS NULL OR p.team_id = p_team_id)
                   AND (p_client_id IS NULL OR p.client_id = p_client_id)
                   AND (p_owner_user_id IS NULL OR p.owner_user_id = p_owner_user_id)
                   AND (NOT p_overdue_only OR p.stage IN ('NEW', 'ACCEPTED', 'ACTIVE', 'ON_HOLD'))
                   AND (p_due_from IS NULL OR p.due_date >= p_due_from)
                   AND (p_due_to IS NULL OR p.due_date <= p_due_to)
                   AND (p_after_due_date IS NULL OR (p.due_date, p.id) > (p_after_due_date, p_after_id))
                 ORDER BY p.due_date, p.id
                 LIMIT v_n)
                UNION
                (SELECT t.id, t.due_date
                 FROM crm.team_members AS tm
                 CROSS JOIN LATERAL (
                     SELECT p.id, p.due_date
                     FROM crm.projects AS p
                     WHERE p.team_id = tm.team_id
                       AND (p_stage IS NULL OR p.stage = p_stage)
                       AND (p_team_id IS NULL OR p.team_id = p_team_id)
                       AND (p_client_id IS NULL OR p.client_id = p_client_id)
                       AND (p_owner_user_id IS NULL OR p.owner_user_id = p_owner_user_id)
                       AND (NOT p_overdue_only OR p.stage IN ('NEW', 'ACCEPTED', 'ACTIVE', 'ON_HOLD'))
                       AND (p_due_from IS NULL OR p.due_date >= p_due_from)
                       AND (p_due_to IS NULL OR p.due_date <= p_due_to)
                       AND (p_after_due_date IS NULL OR (p.due_date, p.id) > (p_after_due_date, p_after_id))
                     ORDER BY p.due_date, p.id
                     LIMIT v_n) AS t
                 WHERE tm.user_id = p_actor_id)
                UNION
                (SELECT cp.id, cp.due_date
                 FROM crm.clients AS c
                 CROSS JOIN LATERAL (
                     SELECT p.id, p.due_date
                     FROM crm.projects AS p
                     WHERE p.client_id = c.id
                       AND (p_stage IS NULL OR p.stage = p_stage)
                       AND (p_team_id IS NULL OR p.team_id = p_team_id)
                       AND (p_owner_user_id IS NULL OR p.owner_user_id = p_owner_user_id)
                       AND (NOT p_overdue_only OR p.stage IN ('NEW', 'ACCEPTED', 'ACTIVE', 'ON_HOLD'))
                       AND (p_due_from IS NULL OR p.due_date >= p_due_from)
                       AND (p_due_to IS NULL OR p.due_date <= p_due_to)
                       AND (p_after_due_date IS NULL OR (p.due_date, p.id) > (p_after_due_date, p_after_id))
                     ORDER BY p.due_date, p.id
                     LIMIT v_n) AS cp
                 WHERE c.assigned_user_id = p_actor_id
                   AND (p_client_id IS NULL OR c.id = p_client_id))
            ),
                 page AS (SELECT cd.id, cd.due_date FROM candidates AS cd ORDER BY cd.due_date, cd.id LIMIT v_n)
            SELECT v.id, v.name, v.stage, v.start_date, v.due_date, v.budget, v.is_overdue, v.stage_changed_at,
               v.client_id, v.client_name, v.client_city, v.team_id, v.team_name, v.owner_user_id, v.owner_login,
               v.owner_display_name, v.updated_at, v.row_version
            FROM page
            JOIN crm.v_projects AS v ON v.id = page.id
            ORDER BY v.due_date, v.id;
        RETURN;
    END IF;

    IF p_overdue_only THEN
        -- Literal stage list = the predicate of the partial index ix_projects_open_due.
        RETURN QUERY
            SELECT v.id, v.name, v.stage, v.start_date, v.due_date, v.budget, v.is_overdue, v.stage_changed_at,
               v.client_id, v.client_name, v.client_city, v.team_id, v.team_name, v.owner_user_id, v.owner_login,
               v.owner_display_name, v.updated_at, v.row_version
            FROM crm.v_projects AS v
            WHERE v.stage IN ('NEW', 'ACCEPTED', 'ACTIVE', 'ON_HOLD')
              AND (p_stage IS NULL OR v.stage = p_stage)
              AND (p_team_id IS NULL OR v.team_id = p_team_id)
              AND (p_client_id IS NULL OR v.client_id = p_client_id)
              AND (p_owner_user_id IS NULL OR v.owner_user_id = p_owner_user_id)
              AND (p_due_from IS NULL OR v.due_date >= p_due_from)
              AND v.due_date <= p_due_to
              AND (p_after_due_date IS NULL OR (v.due_date, v.id) > (p_after_due_date, p_after_id))
            ORDER BY v.due_date, v.id
            LIMIT v_n;
        RETURN;
    END IF;

    RETURN QUERY
        SELECT v.id, v.name, v.stage, v.start_date, v.due_date, v.budget, v.is_overdue, v.stage_changed_at,
               v.client_id, v.client_name, v.client_city, v.team_id, v.team_name, v.owner_user_id, v.owner_login,
               v.owner_display_name, v.updated_at, v.row_version
        FROM crm.v_projects AS v
        WHERE (p_stage IS NULL OR v.stage = p_stage)
          AND (p_team_id IS NULL OR v.team_id = p_team_id)
          AND (p_client_id IS NULL OR v.client_id = p_client_id)
          AND (p_owner_user_id IS NULL OR v.owner_user_id = p_owner_user_id)
          AND (p_due_from IS NULL OR v.due_date >= p_due_from)
          AND (p_due_to IS NULL OR v.due_date <= p_due_to)
          AND (p_after_due_date IS NULL OR (v.due_date, v.id) > (p_after_due_date, p_after_id))
        ORDER BY v.due_date, v.id
        LIMIT v_n;
END;
$$;

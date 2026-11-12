-- One project from crm.v_projects (also what the project procedures return to the API after a write).
CREATE OR REPLACE FUNCTION app.project_get(p_actor_id int, p_id bigint)
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
AS
$$
DECLARE
    v_actor_role text := crm.actor_require(p_actor_id);
BEGIN
    PERFORM crm.project_require_visible(p_actor_id, v_actor_role, p_id);

    RETURN QUERY
        SELECT v.id, v.name, v.stage, v.start_date, v.due_date, v.budget, v.is_overdue, v.stage_changed_at,
               v.client_id, v.client_name, v.client_city, v.team_id, v.team_name, v.owner_user_id, v.owner_login,
               v.owner_display_name, v.updated_at, v.row_version
        FROM crm.v_projects AS v
        WHERE v.id = p_id;
END;
$$;

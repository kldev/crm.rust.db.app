-- Project details (not the stage: see app.project_change_stage). Optimistic concurrency with p_row_version;
-- identical values = no write, same rowVersion. Closed and declined projects are read-only (U1902).
CREATE OR REPLACE PROCEDURE app.project_update(
    p_actor_id      int,
    p_id            bigint,
    p_team_id       int,
    p_owner_user_id int,
    p_name          text,
    p_start_date    date,
    p_due_date      date,
    p_budget        numeric,
    p_row_version   text
)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_actor_role text := crm.actor_require(p_actor_id);
    v_current    record;
BEGIN
    PERFORM crm.project_require_visible(p_actor_id, v_actor_role, p_id);

    p_name := crm.trim_to_null(p_name);
    PERFORM crm.project_validate(p_name, p_team_id, p_owner_user_id, p_start_date, p_due_date, p_budget);

    SELECT p.*, crm.row_version(p.xmin) AS rv INTO v_current FROM crm.projects AS p WHERE p.id = p_id FOR UPDATE;

    IF v_current.rv IS DISTINCT FROM lower(p_row_version) THEN
        RAISE EXCEPTION 'The project was changed by someone else. Reload it and try again.' USING ERRCODE = 'U1901';
    END IF;
    IF v_current.stage IN ('CLOSED', 'DECLINED') THEN
        RAISE EXCEPTION 'A closed or declined project cannot be changed.' USING ERRCODE = 'U1902';
    END IF;

    IF (v_current.team_id, v_current.owner_user_id, v_current.name, v_current.start_date, v_current.due_date,
        v_current.budget)
        IS NOT DISTINCT FROM (p_team_id, p_owner_user_id, p_name, p_start_date, p_due_date, p_budget) THEN
        RETURN;
    END IF;

    UPDATE crm.projects AS p
    SET team_id       = p_team_id,
        owner_user_id = p_owner_user_id,
        name          = p_name,
        start_date    = p_start_date,
        due_date      = p_due_date,
        budget        = p_budget,
        updated_at    = now(),
        updated_by    = p_actor_id
    WHERE p.id = p_id;

    PERFORM crm.team_stats_move(v_current.team_id, v_current.stage, v_current.budget,
                                p_team_id, v_current.stage, p_budget);
END;
$$;

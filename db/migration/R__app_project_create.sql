-- New project in stage NEW. SALES creates projects only for own clients (otherwise U1401, as if the client
-- did not exist). The first row of the stage history (NULL -> NEW) and the team statistics are written in the
-- same transaction.
CREATE OR REPLACE PROCEDURE app.project_create(
    p_actor_id      int,
    p_client_id     bigint,
    p_team_id       int,
    p_owner_user_id int,
    p_name          text,
    p_start_date    date,
    p_due_date      date,
    p_budget        numeric,
    OUT p_id        bigint
)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_actor_role text := crm.actor_require(p_actor_id);
BEGIN
    p_name := crm.trim_to_null(p_name);

    IF NOT EXISTS (SELECT 1
                   FROM crm.clients AS c
                   WHERE c.id = p_client_id
                     AND (v_actor_role <> 'SALES' OR c.assigned_user_id = p_actor_id)) THEN
        RAISE EXCEPTION 'Client not found.' USING ERRCODE = 'U1401';
    END IF;

    PERFORM crm.project_validate(p_name, p_team_id, p_owner_user_id, p_start_date, p_due_date, p_budget);

    INSERT INTO crm.projects (client_id, team_id, owner_user_id, name, stage, start_date, due_date, budget,
                              created_by, updated_by)
    VALUES (p_client_id, p_team_id, p_owner_user_id, p_name, 'NEW', p_start_date, p_due_date, p_budget,
            p_actor_id, p_actor_id)
    RETURNING id INTO p_id;

    INSERT INTO crm.project_stage_history (project_id, from_stage, to_stage, changed_by)
    VALUES (p_id, NULL, 'NEW', p_actor_id);

    PERFORM crm.team_stats_move(NULL, NULL, NULL, p_team_id, 'NEW', p_budget);
END;
$$;

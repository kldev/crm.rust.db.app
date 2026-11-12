CREATE OR REPLACE PROCEDURE app.team_create(p_actor_id int, p_name text, p_lead_user_id int, OUT p_id int)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_actor_role text := crm.actor_require(p_actor_id);
BEGIN
    IF v_actor_role <> 'ADMIN' THEN
        RAISE EXCEPTION 'You are not allowed to perform this operation.' USING ERRCODE = 'U1301';
    END IF;

    p_name := crm.trim_to_null(p_name);
    IF p_name IS NULL OR length(p_name) > 200 THEN
        RAISE EXCEPTION 'Team name is required.' USING ERRCODE = 'U1036';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM crm.users AS u WHERE u.id = p_lead_user_id AND u.role = 'MANAGER' AND u.is_active) THEN
        RAISE EXCEPTION 'A team lead must be an active manager.' USING ERRCODE = 'U1035';
    END IF;

    -- A duplicate name (any case) fails on uq_teams_name (23505) and becomes 409 in the API.
    INSERT INTO crm.teams (name, lead_user_id, created_by)
    VALUES (p_name, p_lead_user_id, p_actor_id)
    RETURNING id INTO p_id;
END;
$$;

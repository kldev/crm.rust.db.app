-- ADMIN or the lead of the team adds an active user. Already a member = no change.
CREATE OR REPLACE PROCEDURE app.team_member_add(p_actor_id int, p_team_id int, p_user_id int)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_actor_role   text := crm.actor_require(p_actor_id);
    v_lead_user_id int;
BEGIN
    SELECT t.lead_user_id INTO v_lead_user_id FROM crm.teams AS t WHERE t.id = p_team_id;
    IF v_lead_user_id IS NULL THEN
        RAISE EXCEPTION 'Team not found.' USING ERRCODE = 'U1404';
    END IF;
    IF v_actor_role <> 'ADMIN' AND v_lead_user_id <> p_actor_id THEN
        RAISE EXCEPTION 'You are not allowed to perform this operation.' USING ERRCODE = 'U1301';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM crm.users AS u WHERE u.id = p_user_id AND u.is_active) THEN
        RAISE EXCEPTION 'Only an active user can join a team.' USING ERRCODE = 'U1038';
    END IF;

    INSERT INTO crm.team_members (team_id, user_id)
    VALUES (p_team_id, p_user_id)
    ON CONFLICT (team_id, user_id) DO NOTHING;
END;
$$;

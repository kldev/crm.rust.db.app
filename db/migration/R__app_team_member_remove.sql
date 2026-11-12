-- ADMIN or the team lead removes a member. A member who still owns open projects of the team stays (U1905).
CREATE OR REPLACE PROCEDURE app.team_member_remove(p_actor_id int, p_team_id int, p_user_id int)
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
    IF EXISTS (SELECT 1
               FROM crm.projects AS p
               WHERE p.owner_user_id = p_user_id
                 AND p.team_id = p_team_id
                 AND p.stage IN ('NEW', 'ACCEPTED', 'ACTIVE', 'ON_HOLD')) THEN
        RAISE EXCEPTION 'The user still owns open projects of this team. Reassign them first.' USING ERRCODE = 'U1905';
    END IF;

    DELETE FROM crm.team_members AS tm WHERE tm.team_id = p_team_id AND tm.user_id = p_user_id;
END;
$$;

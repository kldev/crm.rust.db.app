CREATE OR REPLACE FUNCTION app.team_member_list(p_actor_id int, p_team_id int)
    RETURNS TABLE
            (
                user_id      int,
                login        text,
                display_name text,
                role         text,
                is_active    boolean,
                joined_at    timestamptz
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
        SELECT tm.user_id, u.login, u.display_name, u.role, u.is_active, tm.joined_at
        FROM crm.team_members AS tm
        JOIN crm.users AS u ON u.id = tm.user_id
        WHERE tm.team_id = p_team_id
        ORDER BY u.login;
END;
$$;

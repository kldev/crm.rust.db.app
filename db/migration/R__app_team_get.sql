-- One team, as app.team_create returns it to the API.
CREATE OR REPLACE FUNCTION app.team_get(p_actor_id int, p_id int)
    RETURNS TABLE
            (
                id           int,
                name         text,
                lead_user_id int,
                lead_login   text,
                member_count int,
                created_at   timestamptz
            )
    LANGUAGE plpgsql
    STABLE
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
BEGIN
    PERFORM crm.actor_require(p_actor_id);

    RETURN QUERY
        SELECT t.id, t.name, t.lead_user_id, u.login,
               (SELECT count(*)::int FROM crm.team_members AS tm WHERE tm.team_id = t.id),
               t.created_at
        FROM crm.teams AS t
        JOIN crm.users AS u ON u.id = t.lead_user_id
        WHERE t.id = p_id;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'Team not found.' USING ERRCODE = 'U1404';
    END IF;
END;
$$;

-- Teams with lead and member count, keyset by id. Every active user may list teams (to pick a project team).
CREATE OR REPLACE FUNCTION app.team_list(p_actor_id int, p_after_id int, p_page_size int)
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
    IF p_page_size IS NULL OR p_page_size NOT BETWEEN 1 AND 200 THEN
        RAISE EXCEPTION 'Page size must be between 1 and 200.' USING ERRCODE = 'U1014';
    END IF;

    RETURN QUERY
        SELECT t.id, t.name, t.lead_user_id, u.login,
               (SELECT count(*)::int FROM crm.team_members AS tm WHERE tm.team_id = t.id),
               t.created_at
        FROM crm.teams AS t
        JOIN crm.users AS u ON u.id = t.lead_user_id
        WHERE (p_after_id IS NULL OR t.id > p_after_id)
        ORDER BY t.id
        LIMIT p_page_size + 1;
END;
$$;

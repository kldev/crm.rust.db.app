-- Agents (active CRM users) with the number of tickets each one has in progress. By name, keyset (name, id);
-- p_search is the beginning of the name or the login (any case). Also the list to hand a ticket over to.
CREATE OR REPLACE FUNCTION app.desk_agent_list(p_actor_id int, p_search text, p_after_name text, p_after_id int,
                                               p_page_size int)
    RETURNS TABLE
            (
                id           int,
                login        text,
                display_name text,
                role         text,
                in_progress  int
            )
    LANGUAGE plpgsql
    STABLE
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_pattern text;
BEGIN
    PERFORM crm.actor_require(p_actor_id);
    IF p_page_size IS NULL OR p_page_size NOT BETWEEN 1 AND 200 THEN
        RAISE EXCEPTION 'Page size must be between 1 and 200.' USING ERRCODE = 'U1014';
    END IF;
    p_search := crm.trim_to_null(p_search);
    v_pattern := CASE WHEN p_search IS NOT NULL THEN crm.prefix_pattern(p_search) END;

    RETURN QUERY
        SELECT u.id, u.login, u.display_name, u.role,
               (SELECT count(*)::int
                FROM crm.tickets AS t
                WHERE t.status = 'IN_PROGRESS'
                  AND t.assignee_user_id = u.id)
        FROM crm.users AS u
        WHERE u.is_active
          AND (v_pattern IS NULL OR lower(u.display_name) LIKE v_pattern OR u.login LIKE v_pattern)
          AND (p_after_name IS NULL OR (u.display_name, u.id) > (p_after_name, p_after_id))
        ORDER BY u.display_name, u.id
        LIMIT p_page_size + 1;
END;
$$;

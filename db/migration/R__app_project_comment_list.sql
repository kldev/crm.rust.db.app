-- Comments of a project, newest first, keyset by id (next page: p_before_id = last id of the previous page).
CREATE OR REPLACE FUNCTION app.project_comment_list(p_actor_id int, p_project_id bigint, p_before_id bigint,
                                                    p_page_size int)
    RETURNS TABLE
            (
                id                  bigint,
                project_id          bigint,
                author_id           int,
                author_login        text,
                author_display_name text,
                body                text,
                created_at          timestamptz
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
    PERFORM crm.project_require_visible(p_actor_id, v_actor_role, p_project_id);
    IF p_page_size IS NULL OR p_page_size NOT BETWEEN 1 AND 200 THEN
        RAISE EXCEPTION 'Page size must be between 1 and 200.' USING ERRCODE = 'U1014';
    END IF;

    RETURN QUERY
        SELECT c.id, c.project_id, c.author_id, u.login, u.display_name, c.body, c.created_at
        FROM crm.project_comments AS c
        JOIN crm.users AS u ON u.id = c.author_id
        WHERE c.project_id = p_project_id
          AND (p_before_id IS NULL OR c.id < p_before_id)
        ORDER BY c.id DESC
        LIMIT p_page_size + 1;
END;
$$;

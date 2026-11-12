-- One comment, as app.project_comment_add returns it to the API.
CREATE OR REPLACE FUNCTION app.project_comment_get(p_actor_id int, p_project_id bigint, p_id bigint)
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

    RETURN QUERY
        SELECT c.id, c.project_id, c.author_id, u.login, u.display_name, c.body, c.created_at
        FROM crm.project_comments AS c
        JOIN crm.users AS u ON u.id = c.author_id
        WHERE c.project_id = p_project_id
          AND c.id = p_id;
END;
$$;

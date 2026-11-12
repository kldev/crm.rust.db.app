CREATE OR REPLACE PROCEDURE app.project_comment_add(p_actor_id int, p_project_id bigint, p_body text,
                                                    OUT p_id bigint)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_actor_role text := crm.actor_require(p_actor_id);
BEGIN
    PERFORM crm.project_require_visible(p_actor_id, v_actor_role, p_project_id);

    p_body := crm.trim_to_null(p_body);
    IF p_body IS NULL OR length(p_body) > 4000 THEN
        RAISE EXCEPTION 'Comment text is required.' USING ERRCODE = 'U1034';
    END IF;

    INSERT INTO crm.project_comments (project_id, author_id, body)
    VALUES (p_project_id, p_actor_id, p_body)
    RETURNING id INTO p_id;
END;
$$;

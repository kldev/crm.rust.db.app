CREATE OR REPLACE FUNCTION app.project_stage_history_list(p_actor_id int, p_project_id bigint)
    RETURNS TABLE
            (
                id               bigint,
                from_stage       text,
                to_stage         text,
                changed_at       timestamptz,
                changed_by       int,
                changed_by_login text
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
        SELECT h.id, h.from_stage, h.to_stage, h.changed_at, h.changed_by, u.login
        FROM crm.project_stage_history AS h
        LEFT JOIN crm.users AS u ON u.id = h.changed_by
        WHERE h.project_id = p_project_id
        ORDER BY h.id DESC;
END;
$$;

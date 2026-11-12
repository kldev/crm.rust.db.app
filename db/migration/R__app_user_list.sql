-- Users page (keyset by id). ADMIN and MANAGER (who assigns clients to SALES users).
-- page_size + 1 rows: the caller knows whether there is a next page.
CREATE OR REPLACE FUNCTION app.user_list(p_actor_id int, p_role text, p_after_id int, p_page_size int)
    RETURNS TABLE
            (
                id            int,
                login         text,
                display_name  text,
                role          text,
                is_active     boolean,
                locked_until  timestamptz,
                last_login_at timestamptz,
                created_at    timestamptz,
                updated_at    timestamptz,
                row_version   text
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
    IF v_actor_role NOT IN ('ADMIN', 'MANAGER') THEN
        RAISE EXCEPTION 'You are not allowed to perform this operation.' USING ERRCODE = 'U1301';
    END IF;
    IF p_page_size IS NULL OR p_page_size NOT BETWEEN 1 AND 200 THEN
        RAISE EXCEPTION 'Page size must be between 1 and 200.' USING ERRCODE = 'U1014';
    END IF;
    IF p_role IS NOT NULL AND p_role NOT IN ('ADMIN', 'MANAGER', 'SALES') THEN
        RAISE EXCEPTION 'Role must be ADMIN, MANAGER or SALES.' USING ERRCODE = 'U1016';
    END IF;

    RETURN QUERY
        SELECT u.id, u.login, u.display_name, u.role, u.is_active, u.locked_until, u.last_login_at,
               u.created_at, u.updated_at, crm.row_version(u.xmin)
        FROM crm.users u
        WHERE (p_role IS NULL OR u.role = p_role)
          AND (p_after_id IS NULL OR u.id > p_after_id)
        ORDER BY u.id
        LIMIT p_page_size + 1;
END;
$$;

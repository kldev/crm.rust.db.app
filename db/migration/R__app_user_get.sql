-- One user, as the user procedures return it to the API after a write (ADMIN, MANAGER).
CREATE OR REPLACE FUNCTION app.user_get(p_actor_id int, p_id int)
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

    RETURN QUERY
        SELECT u.id, u.login, u.display_name, u.role, u.is_active, u.locked_until, u.last_login_at,
               u.created_at, u.updated_at, crm.row_version(u.xmin)
        FROM crm.users u
        WHERE u.id = p_id;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'User not found.' USING ERRCODE = 'U1402';
    END IF;
END;
$$;

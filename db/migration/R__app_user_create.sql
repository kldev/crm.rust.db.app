CREATE OR REPLACE PROCEDURE app.user_create(
    p_actor_id      int,
    p_login         text,
    p_display_name  text,
    p_role          text,
    p_password_hash text,
    OUT p_id        int
)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_actor_role text := crm.actor_require(p_actor_id);
BEGIN
    IF v_actor_role <> 'ADMIN' THEN
        RAISE EXCEPTION 'You are not allowed to perform this operation.' USING ERRCODE = 'U1301';
    END IF;

    p_login := lower(btrim(p_login));
    p_display_name := nullif(btrim(p_display_name), '');

    IF p_login IS NULL OR p_login !~ '^[a-z0-9._-]{3,64}$' THEN
        RAISE EXCEPTION 'Login must have 3-64 characters: letters, digits, dot, underscore or hyphen.'
            USING ERRCODE = 'U1015';
    END IF;
    IF p_display_name IS NULL OR length(p_display_name) > 200 THEN
        RAISE EXCEPTION 'Display name is required.' USING ERRCODE = 'U1019';
    END IF;
    IF p_role IS NULL OR p_role NOT IN ('ADMIN', 'MANAGER', 'SALES') THEN
        RAISE EXCEPTION 'Role must be ADMIN, MANAGER or SALES.' USING ERRCODE = 'U1016';
    END IF;
    IF p_password_hash IS NULL OR length(p_password_hash) = 0 THEN
        RAISE EXCEPTION 'Password is required.' USING ERRCODE = 'U1020';
    END IF;

    -- A duplicate login fails on uq_users_login (23505) and becomes 409 in the API.
    INSERT INTO crm.users (login, display_name, role, password_hash, created_by, updated_by)
    VALUES (p_login, p_display_name, p_role, p_password_hash, p_actor_id, p_actor_id)
    RETURNING id INTO p_id;
END;
$$;

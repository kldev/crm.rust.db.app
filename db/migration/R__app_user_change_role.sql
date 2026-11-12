CREATE OR REPLACE PROCEDURE app.user_change_role(p_actor_id int, p_id int, p_role text)
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
    IF p_role IS NULL OR p_role NOT IN ('ADMIN', 'MANAGER', 'SALES') THEN
        RAISE EXCEPTION 'Role must be ADMIN, MANAGER or SALES.' USING ERRCODE = 'U1016';
    END IF;
    IF p_id = p_actor_id THEN
        RAISE EXCEPTION 'You cannot change your own role or deactivate yourself.' USING ERRCODE = 'U1018';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM crm.users u WHERE u.id = p_id) THEN
        RAISE EXCEPTION 'User not found.' USING ERRCODE = 'U1402';
    END IF;

    -- Same role = no change, no write, same rowVersion.
    UPDATE crm.users
    SET role       = p_role,
        updated_at = now(),
        updated_by = p_actor_id
    WHERE id = p_id
      AND role IS DISTINCT FROM p_role;
END;
$$;

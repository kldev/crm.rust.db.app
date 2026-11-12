-- New password hash (computed by the application). Own password (the application has verified the current one)
-- or any user's password when the actor is ADMIN. Clears a lockout.
CREATE OR REPLACE PROCEDURE app.user_set_password(p_actor_id int, p_id int, p_password_hash text)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_actor_role text := crm.actor_require(p_actor_id);
BEGIN
    IF p_id IS DISTINCT FROM p_actor_id AND v_actor_role <> 'ADMIN' THEN
        RAISE EXCEPTION 'You are not allowed to perform this operation.' USING ERRCODE = 'U1301';
    END IF;
    IF p_password_hash IS NULL OR length(p_password_hash) = 0 THEN
        RAISE EXCEPTION 'Password is required.' USING ERRCODE = 'U1020';
    END IF;

    UPDATE crm.users
    SET password_hash      = p_password_hash,
        failed_login_count = 0,
        locked_until       = NULL,
        updated_at         = now(),
        updated_by         = p_actor_id
    WHERE id = p_id;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'User not found.' USING ERRCODE = 'U1402';
    END IF;
END;
$$;

-- Bootstrap of the first administrator at application start (CRM_ADMIN_LOGIN / CRM_ADMIN_PASSWORD).
-- Creates the account only when there is no ADMIN at all; otherwise does nothing.
CREATE OR REPLACE PROCEDURE app.user_ensure_admin(
    p_login         text,
    p_display_name  text,
    p_password_hash text,
    OUT p_created   boolean
)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
BEGIN
    p_created := false;
    p_login := lower(btrim(p_login));

    IF p_login IS NULL OR p_login !~ '^[a-z0-9._-]{3,64}$' THEN
        RAISE EXCEPTION 'Login must have 3-64 characters: letters, digits, dot, underscore or hyphen.'
            USING ERRCODE = 'U1015';
    END IF;
    IF p_password_hash IS NULL OR length(p_password_hash) = 0 THEN
        RAISE EXCEPTION 'Password is required.' USING ERRCODE = 'U1020';
    END IF;

    -- Two instances starting at once: one creates the administrator, the other sees it.
    PERFORM pg_advisory_xact_lock(hashtext('crm.user_ensure_admin'));

    IF NOT EXISTS (SELECT 1 FROM crm.users u WHERE u.role = 'ADMIN') THEN
        INSERT INTO crm.users (login, display_name, role, password_hash)
        VALUES (p_login, coalesce(nullif(btrim(p_display_name), ''), p_login), 'ADMIN', p_password_hash);
        p_created := true;
    END IF;
END;
$$;

CREATE OR REPLACE PROCEDURE app.user_login_succeeded(p_user_id int)
    LANGUAGE sql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
UPDATE crm.users
SET failed_login_count = 0,
    locked_until       = NULL,
    last_login_at      = now()
WHERE id = p_user_id;
$$;

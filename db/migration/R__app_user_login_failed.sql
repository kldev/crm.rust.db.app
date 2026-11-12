-- Counts a failed login. After p_max_failed failures in a row the account is locked for p_lock_minutes.
-- Unknown login: nothing happens (and the caller answers exactly like for a wrong password).
CREATE OR REPLACE PROCEDURE app.user_login_failed(p_login text, p_max_failed int, p_lock_minutes int)
    LANGUAGE sql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
UPDATE crm.users
SET failed_login_count = CASE WHEN failed_login_count + 1 >= p_max_failed THEN 0 ELSE failed_login_count + 1 END,
    locked_until       = CASE
                             WHEN failed_login_count + 1 >= p_max_failed
                                 THEN now() + make_interval(mins => p_lock_minutes)
                             ELSE locked_until
                         END
WHERE login = lower(btrim(p_login));
$$;

-- Data for the login: the hash is verified in the application (bcrypt), never in SQL.
CREATE OR REPLACE FUNCTION app.user_get_for_login(p_login text)
    RETURNS TABLE
            (
                id            int,
                login         text,
                display_name  text,
                role          text,
                password_hash text,
                is_active     boolean,
                locked_until  timestamptz
            )
    LANGUAGE sql
    STABLE
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
SELECT u.id, u.login, u.display_name, u.role, u.password_hash, u.is_active, u.locked_until
FROM crm.users u
WHERE u.login = lower(btrim(p_login));
$$;

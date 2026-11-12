-- Readiness check of the database through the same path as every other call (schema app).
CREATE OR REPLACE FUNCTION app.health_ping()
    RETURNS TABLE (ok boolean, server_time timestamptz)
    LANGUAGE sql
    STABLE
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
SELECT true, now();
$$;

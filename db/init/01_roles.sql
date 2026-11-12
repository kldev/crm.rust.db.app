-- Server bootstrap (CRM_MODE=init, superuser), run before the migrations. Idempotent: safe on every start.
-- Placeholders ${...} are filled by the binary; passwords arrive already quoted as SQL literals.
--   crm_owner       NOLOGIN, owns every object (schemas crm, app, seed); procedures run as it (SECURITY DEFINER).
--   ${migrator}     LOGIN, member of crm_owner: migrations and the seed (CRM_MODE=migrate|seed).
--   ${app_user}     LOGIN, the API: USAGE on schema app, EXECUTE on its routines, SELECT on its views. Nothing else.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'crm_owner') THEN
        CREATE ROLE crm_owner NOLOGIN;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = '${migrator}') THEN
        CREATE ROLE "${migrator}" LOGIN;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = '${app_user}') THEN
        CREATE ROLE "${app_user}" LOGIN;
    END IF;
END
$$;

ALTER ROLE "${migrator}" WITH LOGIN PASSWORD ${migrator_password};
ALTER ROLE "${app_user}" WITH LOGIN NOINHERIT PASSWORD ${app_password};
GRANT crm_owner TO "${migrator}";

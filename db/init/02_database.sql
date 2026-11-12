-- Run inside the CRM database (superuser), after it was created with OWNER crm_owner. Idempotent.
REVOKE ALL ON DATABASE "${database}" FROM PUBLIC;
GRANT CONNECT, TEMPORARY ON DATABASE "${database}" TO "${migrator}", "${app_user}";

-- Nobody but the owner creates objects in public; the application uses only schema app.
REVOKE ALL ON SCHEMA public FROM PUBLIC;

-- Migration history (CRM_MODE=migrate), outside the schemas the migrations create.
CREATE SCHEMA IF NOT EXISTS migration AUTHORIZATION crm_owner;
CREATE TABLE IF NOT EXISTS migration.history
(
    script      text        NOT NULL PRIMARY KEY,
    version     int         NULL,
    checksum    text        NOT NULL,
    applied_at  timestamptz NOT NULL DEFAULT now(),
    duration_ms int         NOT NULL
);
ALTER TABLE migration.history OWNER TO crm_owner;

-- Routines are not executable by PUBLIC (PostgreSQL's default); the application gets them per schema (V1).
ALTER DEFAULT PRIVILEGES FOR ROLE crm_owner REVOKE EXECUTE ON ROUTINES FROM PUBLIC;

-- Statement statistics for perf/sql (the server preloads the library in the compose stack). Created here, not in
-- a migration: it is a tool of the operator, not something the application uses.
CREATE EXTENSION IF NOT EXISTS pg_stat_statements;

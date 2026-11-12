-- ADR-0013: tables live in `crm`, the application reaches only `app` (procedures, functions and views),
-- the seed lives in `seed`. Objects are owned by crm_owner (the migrator runs SET ROLE crm_owner).
CREATE SCHEMA crm;
CREATE SCHEMA app;
CREATE SCHEMA seed;

GRANT USAGE ON SCHEMA app TO "${app_user}";
ALTER DEFAULT PRIVILEGES FOR ROLE crm_owner IN SCHEMA app GRANT EXECUTE ON ROUTINES TO "${app_user}";
ALTER DEFAULT PRIVILEGES FOR ROLE crm_owner IN SCHEMA app GRANT SELECT ON TABLES TO "${app_user}";

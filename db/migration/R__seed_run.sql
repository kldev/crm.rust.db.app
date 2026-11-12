/*
  The one data seed of this project (ADR-0004): users, teams, clients, projects and comments for performance tests.
  Run by CRM_MODE=seed with the migrator login (as crm_owner); the application login cannot reach schema `seed`.

  Set-based: every table is one INSERT ... SELECT over generate_series, in statements of p_batch_size rows; no row
  loops. One transaction: the secondary indexes of the loaded tables are dropped first and built again at the end
  (with their exact definitions from the catalog), then ANALYZE - so a failure leaves the database as it was.
  Deterministic: every value comes from sha256('<seed>:<kind>:<n>') cut into 4-byte numbers, the same numbers as
  the T-SQL seed (SHA2_256 + CAST(SUBSTRING(...) AS int) & 0x7FFFFFFF), so both databases hold the same data.
  Ids are given explicitly from each table's identity sequence, so row n always gets the n-th id.
  Refuses to run twice (U1060): to seed again, start from an empty database (start.sh --clean).

  Users:    perf.admin.0001.., perf.manager.0001.., perf.sales.0001.. - all with the same p_password_hash.
  Teams:    ceil(sales / team_size) teams, led by managers (round robin); SALES user k is in team k % teams.
  Clients:  assigned to SALES users (spread by hash); names are "<adjective> <noun> <legal form>" from short lists,
            so a name prefix (e.g. "Zielony") matches about 1/40 of all clients.
  Projects: for a client chosen by hash, owned by the client's sales user and delivered by that user's team;
            stage, dates and budget by hash; one stage history row each; crm.team_project_stats filled to match.
  Comments: on projects chosen by hash, written by the project owner.
*/

-- One 31-bit number from bytes 4*p+1 .. 4*p+4 of a hash (big-endian, the sign bit dropped).
CREATE OR REPLACE FUNCTION seed.h(p_hash bytea, p_part int)
    RETURNS bigint
    LANGUAGE sql
    IMMUTABLE
    PARALLEL SAFE
AS
$$
SELECT (('x' || encode(substring(p_hash FROM p_part * 4 + 1 FOR 4), 'hex'))::bit(32)::int & 2147483647)::bigint;
$$;

CREATE OR REPLACE FUNCTION seed.hash(p_random_seed int, p_kind text, p_n bigint)
    RETURNS bytea
    LANGUAGE sql
    IMMUTABLE
    PARALLEL SAFE
AS
$$
SELECT sha256(convert_to(p_random_seed::text || ':' || p_kind || ':' || p_n::text, 'UTF8'));
$$;

CREATE OR REPLACE PROCEDURE seed.run(
    p_clients       int,
    p_admins        int,
    p_managers      int,
    p_sales         int,
    p_password_hash text,
    p_team_size     int,
    p_projects      int,
    p_comments      int,
    p_random_seed   int,
    p_batch_size    int,
    INOUT users_created int DEFAULT NULL,
    INOUT teams_created int DEFAULT NULL,
    INOUT clients_created int DEFAULT NULL,
    INOUT projects_created int DEFAULT NULL,
    INOUT comments_created int DEFAULT NULL,
    INOUT seconds int DEFAULT NULL
)
    LANGUAGE plpgsql
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_started        timestamptz(3) := clock_timestamp();
    v_adjectives     text[] := string_to_array(
            'Zielony,Szybki,Nowy,Stary,Wielki,Mały,Jasny,Ciemny,Złoty,Srebrny,Polski,Północny,Południowy,'
                || 'Wschodni,Zachodni,Górski,Morski,Leśny,Miejski,Wiejski,Cichy,Mocny,Lekki,Twardy,Ciepły,Zimny,'
                || 'Prosty,Pewny,Wspólny,Główny,Pierwszy,Drugi,Trzeci,Dobry,Mądry,Wesoły,Spokojny,Bystry,Śmiały,'
                || 'Zwinny', ',');
    v_nouns          text[] := string_to_array(
            'Most,Młyn,Dom,Port,Las,Kamień,Ogród,Rynek,Bór,Gród,Dwór,Zakład,Warsztat,Handel,Transport,'
                || 'Serwis,Projekt,System,Rozwój,Kapitał,Budynek,Sklep,Hurt,Detal,Tartak,Browar,Piekarz,Kowal,'
                || 'Ślusarz,Stolarz,Ogrodnik,Rybak,Rolnik,Kupiec,Inżynier,Architekt,Doradca,Partner,Lider,Mistrz', ',');
    v_forms          text[] := string_to_array('Sp. z o.o.,S.A.,Sp.j.,Sp.k.,s.c.', ',');
    v_cities         text[] := string_to_array(
            'Warszawa,Kraków,Łódź,Wrocław,Poznań,Gdańsk,Szczecin,Bydgoszcz,Lublin,Białystok,Katowice,'
                || 'Gdynia,Częstochowa,Radom,Rzeszów,Toruń,Sosnowiec,Kielce,Gliwice,Olsztyn', ',');
    v_kinds          text[] := string_to_array(
            'Wdrożenie CRM,Migracja danych,Audyt procesów,Szkolenie zespołu,Integracja ERP,Raportowanie,'
                || 'Kampania marketingowa,Serwis posprzedażowy,Rozbudowa sklepu,Optymalizacja kosztów', ',');
    v_phrases        text[] := string_to_array(
            'Spotkanie z klientem odbyte.|Czekamy na akceptację oferty.|Klient prosi o zmianę terminu.|'
                || 'Wysłano harmonogram prac.|Zespół rozpoczął realizację.|Zgłoszono uwagi do zakresu.|'
                || 'Faktura za etap wystawiona.|Ryzyko opóźnienia po stronie klienta.', '|');
    v_first_user     int;
    v_first_manager  int;
    v_first_sales    int;
    v_team_count     int;
    v_first_team     int;
    v_first_client   bigint;
    v_first_project  bigint;
    v_first_history  bigint;
    v_first_comment  bigint;
    v_done           int;
    v_end            int;
    v_indexes        text[];
    v_index_names    text[];
    v_def            text;
BEGIN
    IF p_clients IS NULL OR p_clients < 0 OR p_clients > 40000000 THEN
        RAISE EXCEPTION 'Seed: clients must be between 0 and 40000000.' USING ERRCODE = 'U1050';
    END IF;
    IF p_admins IS NULL OR p_admins < 0 OR p_managers IS NULL OR p_managers < 0 OR p_sales IS NULL OR p_sales < 0 THEN
        RAISE EXCEPTION 'Seed: user counts must be 0 or more.' USING ERRCODE = 'U1050';
    END IF;
    IF p_admins > 9999 OR p_managers > 9999 OR p_sales > 9999 THEN
        RAISE EXCEPTION 'Seed: at most 9999 users per role (login perf.<role>.NNNN).' USING ERRCODE = 'U1050';
    END IF;
    IF p_clients > 0 AND p_sales = 0 THEN
        RAISE EXCEPTION 'Seed: clients need at least one sales user to be assigned to.' USING ERRCODE = 'U1050';
    END IF;
    IF p_projects IS NULL OR p_projects < 0 OR p_projects > 40000000
        OR p_comments IS NULL OR p_comments < 0 OR p_comments > 40000000 THEN
        RAISE EXCEPTION 'Seed: projects and comments must be between 0 and 40000000.' USING ERRCODE = 'U1050';
    END IF;
    IF p_projects > 0 AND (p_clients = 0 OR p_managers = 0) THEN
        RAISE EXCEPTION 'Seed: projects need clients and at least one manager (team lead).' USING ERRCODE = 'U1050';
    END IF;
    IF p_comments > 0 AND p_projects = 0 THEN
        RAISE EXCEPTION 'Seed: comments need projects.' USING ERRCODE = 'U1050';
    END IF;
    IF p_team_size IS NULL OR p_team_size < 1 OR p_team_size > 1000 THEN
        RAISE EXCEPTION 'Seed: team size must be between 1 and 1000.' USING ERRCODE = 'U1050';
    END IF;
    IF p_batch_size IS NULL OR p_batch_size NOT BETWEEN 1000 AND 2000000 THEN
        RAISE EXCEPTION 'Seed: batch size must be between 1000 and 2000000.' USING ERRCODE = 'U1050';
    END IF;
    IF p_random_seed IS NULL THEN
        RAISE EXCEPTION 'Seed: random seed is required.' USING ERRCODE = 'U1050';
    END IF;
    IF p_password_hash IS NULL OR length(p_password_hash) = 0 THEN
        RAISE EXCEPTION 'Password is required.' USING ERRCODE = 'U1020';
    END IF;
    -- One seed at a time; the check below then sees the other one's data.
    LOCK TABLE crm.clients IN EXCLUSIVE MODE;
    IF EXISTS (SELECT 1 FROM crm.users WHERE login LIKE 'perf.%') OR EXISTS (SELECT 1 FROM crm.clients) THEN
        RAISE EXCEPTION 'Seed: the database already has seed data or clients. Start from an empty database (start.sh --clean).'
            USING ERRCODE = 'U1060';
    END IF;

    SET LOCAL maintenance_work_mem = '512MB';

    -- Secondary indexes of the bulk-loaded tables (not those behind a constraint): built once, after the load.
    SELECT coalesce(array_agg(pg_get_indexdef(i.indexrelid) ORDER BY c.relname), '{}'),
           coalesce(array_agg('crm.' || quote_ident(c.relname) ORDER BY c.relname), '{}')
    INTO v_indexes, v_index_names
    FROM pg_index AS i
    JOIN pg_class AS c ON c.oid = i.indexrelid
    WHERE i.indrelid IN ('crm.clients'::regclass, 'crm.projects'::regclass, 'crm.project_comments'::regclass,
                         'crm.project_stage_history'::regclass)
      AND NOT EXISTS (SELECT 1 FROM pg_constraint AS k WHERE k.conindid = i.indexrelid);
    FOREACH v_def IN ARRAY v_index_names
        LOOP
            EXECUTE 'DROP INDEX ' || v_def;
        END LOOP;

    -- Users: admins, then managers, then sales, ids in that order.
    v_first_user := nextval(pg_get_serial_sequence('crm.users', 'id'));
    INSERT INTO crm.users (id, login, display_name, role, password_hash, created_at, updated_at)
    SELECT v_first_user + u.n - 1, u.login, u.display_name, u.role, p_password_hash, v_started, v_started
    FROM (SELECT row_number() OVER (ORDER BY r.role_order, r.v) AS n, r.*
          FROM (SELECT 'perf.admin.' || lpad(v::text, 4, '0') AS login, 'Perf Admin ' || v AS display_name,
                       'ADMIN' AS role, 1 AS role_order, v
                FROM generate_series(1, p_admins) AS v
                UNION ALL
                SELECT 'perf.manager.' || lpad(v::text, 4, '0'), 'Perf Manager ' || v, 'MANAGER', 2, v
                FROM generate_series(1, p_managers) AS v
                UNION ALL
                SELECT 'perf.sales.' || lpad(v::text, 4, '0'), 'Perf Sales ' || v, 'SALES', 3, v
                FROM generate_series(1, p_sales) AS v) AS r) AS u;
    users_created := p_admins + p_managers + p_sales;
    PERFORM setval(pg_get_serial_sequence('crm.users', 'id'), v_first_user + greatest(users_created, 1) - 1);
    v_first_manager := v_first_user + p_admins;
    v_first_sales := v_first_user + p_admins + p_managers;

    v_team_count := CASE WHEN p_managers = 0 THEN 0 ELSE (p_sales + p_team_size - 1) / p_team_size END;
    teams_created := v_team_count;
    IF v_team_count > 0 THEN
        v_first_team := nextval(pg_get_serial_sequence('crm.teams', 'id'));
        INSERT INTO crm.teams (id, name, lead_user_id, created_at)
        SELECT v_first_team + v - 1, 'Perf Team ' || lpad(v::text, 4, '0'), v_first_manager + (v - 1) % p_managers,
               v_started
        FROM generate_series(1, v_team_count) AS v;
        PERFORM setval(pg_get_serial_sequence('crm.teams', 'id'), v_first_team + v_team_count - 1);

        INSERT INTO crm.team_members (team_id, user_id, joined_at)
        SELECT v_first_team + (v - 1) % v_team_count, v_first_sales + v - 1, v_started
        FROM generate_series(1, p_sales) AS v;
    END IF;

    -- Clients.
    v_first_client := nextval(pg_get_serial_sequence('crm.clients', 'id'));
    v_done := 0;
    WHILE v_done < p_clients
        LOOP
            v_end := least(p_clients, v_done + p_batch_size);
            INSERT INTO crm.clients (id, name, nip, email, phone, city, status, assigned_user_id, note, created_at,
                                     updated_at)
            SELECT v_first_client + s.n - 1,
                   v_adjectives[seed.h(s.b, 0) % cardinality(v_adjectives) + 1] || ' '
                       || v_nouns[seed.h(s.b, 1) % cardinality(v_nouns) + 1] || ' '
                       || v_forms[seed.h(s.b, 2) % cardinality(v_forms) + 1],
                   (1000000000 + s.n)::text,
                   'klient' || s.n || '@example.test',
                   '+48 5' || lpad((seed.h(s.b, 3) % 100000000)::text, 8, '0'),
                   -- Skewed cities: the first ones are more frequent (sqrt of h % n^2).
                   v_cities[cardinality(v_cities)
                       - floor(sqrt((seed.h(s.b, 2) % (cardinality(v_cities) * cardinality(v_cities)))::float8))::int],
                   CASE WHEN seed.h(s.b, 4) % 10 < 7 THEN 'ACTIVE'
                        WHEN seed.h(s.b, 4) % 10 < 9 THEN 'LEAD'
                        ELSE 'INACTIVE' END,
                   v_first_sales + (seed.h(s.b, 5) % p_sales)::int,
                   CASE WHEN seed.h(s.b, 6) % 20 = 0 THEN 'Seed note: follow up in the next quarter.' END,
                   v_started - make_interval(mins => (seed.h(s.b, 7) % 1051200)::int),
                   v_started
            FROM (SELECT g AS n, seed.hash(p_random_seed, 'c', g) AS b
                  FROM generate_series(v_done + 1, v_end) AS g) AS s;
            v_done := v_end;
            RAISE NOTICE 'seed: % / % clients', v_done, p_clients;
        END LOOP;
    clients_created := p_clients;
    IF p_clients > 0 THEN
        PERFORM setval(pg_get_serial_sequence('crm.clients', 'id'), v_first_client + p_clients - 1);
    END IF;

    -- Projects: client n -> its sales user k (the same hash as the client's assignment) -> k's team.
    v_first_project := nextval(pg_get_serial_sequence('crm.projects', 'id'));
    v_done := 0;
    WHILE v_done < p_projects
        LOOP
            v_end := least(p_projects, v_done + p_batch_size);
            INSERT INTO crm.projects (id, client_id, team_id, owner_user_id, name, stage, start_date, due_date,
                                      budget, stage_changed_at, created_at, updated_at)
            SELECT v_first_project + x.n - 1,
                   v_first_client + x.client_n - 1,
                   v_first_team + (x.sales_k % v_team_count)::int,
                   v_first_sales + x.sales_k::int,
                   v_kinds[seed.h(x.b, 2) % cardinality(v_kinds) + 1] || ' #' || x.n,
                   CASE WHEN seed.h(x.b, 1) % 20 < 2 THEN 'NEW'
                        WHEN seed.h(x.b, 1) % 20 < 4 THEN 'ACCEPTED'
                        WHEN seed.h(x.b, 1) % 20 < 11 THEN 'ACTIVE'
                        WHEN seed.h(x.b, 1) % 20 < 12 THEN 'ON_HOLD'
                        WHEN seed.h(x.b, 1) % 20 < 18 THEN 'CLOSED'
                        ELSE 'DECLINED' END,
                   x.start_date,
                   x.start_date + 14 + (seed.h(x.b, 3) % 350)::int,
                   (1000 + seed.h(x.b, 4) % 499000)::numeric(19, 2) + (seed.h(x.b, 5) % 100)::numeric(19, 2) / 100,
                   v_started, v_started, v_started
            FROM (SELECT p.n, p.b, p.client_n, p.start_date,
                         seed.h(seed.hash(p_random_seed, 'c', p.client_n), 5) % p_sales AS sales_k
                  FROM (SELECT g AS n, h.b, 1 + seed.h(h.b, 0) % p_clients AS client_n,
                               date '2024-01-01' + (seed.h(h.b, 2) % 1000)::int AS start_date
                        FROM generate_series(v_done + 1, v_end) AS g
                        CROSS JOIN LATERAL (SELECT seed.hash(p_random_seed, 'p', g) AS b) AS h) AS p) AS x;
            v_done := v_end;
            RAISE NOTICE 'seed: % / % projects', v_done, p_projects;
        END LOOP;
    projects_created := p_projects;
    IF p_projects > 0 THEN
        PERFORM setval(pg_get_serial_sequence('crm.projects', 'id'), v_first_project + p_projects - 1);

        v_first_history := nextval(pg_get_serial_sequence('crm.project_stage_history', 'id'));
        INSERT INTO crm.project_stage_history (id, project_id, from_stage, to_stage, changed_at)
        SELECT v_first_history + p.id - v_first_project, p.id, NULL, p.stage, v_started
        FROM crm.projects AS p
        WHERE p.id >= v_first_project;
        PERFORM setval(pg_get_serial_sequence('crm.project_stage_history', 'id'), v_first_history + p_projects - 1);

        -- The team statistics the procedures keep in step (ADR-0015), here in one aggregate.
        INSERT INTO crm.team_project_stats AS s (team_id, stage, project_count, budget_total)
        SELECT p.team_id, p.stage, count(*), sum(p.budget)
        FROM crm.projects AS p
        WHERE p.id >= v_first_project
        GROUP BY p.team_id, p.stage
        ON CONFLICT (team_id, stage) DO UPDATE
            SET project_count = s.project_count + excluded.project_count,
                budget_total  = s.budget_total + excluded.budget_total;
    END IF;

    -- Comments: on a project chosen by hash, by its owner.
    v_first_comment := nextval(pg_get_serial_sequence('crm.project_comments', 'id'));
    v_done := 0;
    WHILE v_done < p_comments
        LOOP
            v_end := least(p_comments, v_done + p_batch_size);
            INSERT INTO crm.project_comments (id, project_id, author_id, body, created_at)
            SELECT v_first_comment + m.n - 1, p.id, p.owner_user_id,
                   v_phrases[seed.h(m.b, 1) % cardinality(v_phrases) + 1],
                   v_started - make_interval(secs => (seed.h(m.b, 1) % 31536000)::int)
            FROM (SELECT g AS n, seed.hash(p_random_seed, 'm', g) AS b
                  FROM generate_series(v_done + 1, v_end) AS g) AS m
            JOIN crm.projects AS p ON p.id = v_first_project + seed.h(m.b, 0) % p_projects;
            v_done := v_end;
            RAISE NOTICE 'seed: % / % comments', v_done, p_comments;
        END LOOP;
    comments_created := p_comments;
    IF p_comments > 0 THEN
        PERFORM setval(pg_get_serial_sequence('crm.project_comments', 'id'), v_first_comment + p_comments - 1);
    END IF;

    RAISE NOTICE 'seed: building % indexes', cardinality(v_indexes);
    FOREACH v_def IN ARRAY v_indexes
        LOOP
            EXECUTE v_def;
        END LOOP;
    -- Fresh statistics after a bulk load, so the first queries get good plans.
    ANALYZE crm.users, crm.teams, crm.team_members, crm.clients, crm.projects, crm.project_stage_history,
        crm.project_comments, crm.team_project_stats;

    seconds := extract(epoch FROM clock_timestamp() - v_started)::int;
END;
$$;

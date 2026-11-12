/*
  Seed of the service desk (ADR-0007), with the migrator login. Idempotent: agents and people are added only when
  missing (by login / e-mail), tickets only into an empty desk - run it again at any time.

  Agents: the desk team as CRM users (role SALES, the same p_password_hash as the perf users); any CRM user can
  work tickets, these ten just have names to recognise. People: the requesters of the public form.
  Tickets: p_tickets reported over the last 28 days through the same functions and procedures the application
  calls, so every ticket has a real stream; about 40% are taken and half of those resolved. Deterministic for one
  p_random_seed (the same sha256 numbers as seed.run).
*/
CREATE OR REPLACE PROCEDURE seed.desk_run(p_password_hash text, p_tickets int, p_random_seed int)
    LANGUAGE plpgsql
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_agents    int[];
    v_subs      text[];
    v_cats      text[];
    v_names     text[];
    v_people    int[];
    v_h         bytea;
    v_sub_n     int;
    v_priority  text;
    v_reported  timestamptz;
    v_agent     int;
    v_person    int;
    v_title     text;
    v_ticket_id bigint;
BEGIN
    IF p_password_hash IS NULL OR length(p_password_hash) = 0 THEN
        RAISE EXCEPTION 'Password is required.' USING ERRCODE = 'U1020';
    END IF;
    IF p_tickets IS NULL OR p_tickets NOT BETWEEN 0 AND 100000 THEN
        RAISE EXCEPTION 'Seed: desk tickets must be between 0 and 100000.' USING ERRCODE = 'U1050';
    END IF;
    IF p_random_seed IS NULL THEN
        RAISE EXCEPTION 'Seed: random seed is required.' USING ERRCODE = 'U1050';
    END IF;

    INSERT INTO crm.users (login, display_name, role, password_hash)
    SELECT a.login, a.display_name, 'SALES', p_password_hash
    FROM (VALUES ('anna.nowak', 'Anna Nowak'),
            ('piotr.kowalski', 'Piotr Kowalski'),
            ('katarzyna.wisniewska', 'Katarzyna Wiśniewska'),
            ('tomasz.wojcik', 'Tomasz Wójcik'),
            ('magdalena.kaminska', 'Magdalena Kamińska'),
            ('michal.lewandowski', 'Michał Lewandowski'),
            ('agnieszka.zielinska', 'Agnieszka Zielińska'),
            ('pawel.szymanski', 'Paweł Szymański'),
            ('joanna.wozniak', 'Joanna Woźniak'),
            ('krzysztof.dabrowski', 'Krzysztof Dąbrowski')) AS a (login, display_name)
    WHERE NOT EXISTS (SELECT 1 FROM crm.users AS u WHERE u.login = a.login);

    INSERT INTO crm.people (first_name, last_name, department, email, phone)
    SELECT p.first_name, p.last_name, p.department, p.email, p.phone
    FROM (VALUES ('Adam', 'Mazur', 'Sprzedaż', 'adam.mazur@company.io', '+48 600 200 201'),
            ('Barbara', 'Krawczyk', 'Sprzedaż', 'barbara.krawczyk@company.io', NULL),
            ('Cezary', 'Piotrowski', 'Sprzedaż', 'cezary.piotrowski@company.io', '+48 600 200 203'),
            ('Dorota', 'Grabowska', 'Finanse', 'dorota.grabowska@company.io', '+48 600 200 204'),
            ('Emil', 'Nowakowski', 'Finanse', 'emil.nowakowski@company.io', NULL),
            ('Fryderyka', 'Pawłowska', 'Finanse', 'fryderyka.pawlowska@company.io', '+48 600 200 206'),
            ('Grzegorz', 'Michalski', 'Kadry', 'grzegorz.michalski@company.io', '+48 600 200 207'),
            ('Halina', 'Nowicka', 'Kadry', 'halina.nowicka@company.io', NULL),
            ('Igor', 'Adamczyk', 'Logistyka', 'igor.adamczyk@company.io', '+48 600 200 209'),
            ('Joanna', 'Dudek', 'Logistyka', 'joanna.dudek@company.io', '+48 600 200 210'),
            ('Kamil', 'Zając', 'Logistyka', 'kamil.zajac@company.io', NULL),
            ('Lena', 'Wieczorek', 'Zarząd', 'lena.wieczorek@company.io', '+48 600 200 212'),
            ('Marek', 'Jabłoński', 'Zarząd', 'marek.jablonski@company.io', '+48 600 200 213'),
            ('Natalia', 'Król', 'Obsługa klienta', 'natalia.krol@company.io', '+48 600 200 214'),
            ('Oskar', 'Majewski', 'Obsługa klienta', 'oskar.majewski@company.io', NULL),
            ('Patrycja', 'Olszewska', 'Obsługa klienta', 'patrycja.olszewska@company.io', '+48 600 200 216'),
            ('Robert', 'Jaworski', 'Produkt', 'robert.jaworski@company.io', '+48 600 200 217'),
            ('Sylwia', 'Wróbel', 'Produkt', 'sylwia.wrobel@company.io', NULL),
            ('Tadeusz', 'Malinowski', 'Produkt', 'tadeusz.malinowski@company.io', '+48 600 200 219'),
            ('Urszula', 'Pawlak', 'Zakupy', 'urszula.pawlak@company.io', '+48 600 200 220'),
            ('Wiktor', 'Witkowski', 'Zakupy', 'wiktor.witkowski@company.io', NULL),
            ('Zofia', 'Walczak', 'Administracja', 'zofia.walczak@company.io', '+48 600 200 222'),
            ('Artur', 'Stępień', 'Administracja', 'artur.stepien@company.io', '+48 600 200 223'),
            ('Beata', 'Górska', 'Badania i rozwój', 'beata.gorska@company.io', NULL),
            ('Daniel', 'Rutkowski', 'Badania i rozwój', 'daniel.rutkowski@company.io', '+48 600 200 225'),
            ('Ewa', 'Michalak', 'Badania i rozwój', 'ewa.michalak@company.io', '+48 600 200 226'),
            ('Filip', 'Sikora', 'Jakość', 'filip.sikora@company.io', NULL),
            ('Gabriela', 'Ostrowska', 'Jakość', 'gabriela.ostrowska@company.io', '+48 600 200 228'),
            ('Henryk', 'Baran', 'Utrzymanie ruchu', 'henryk.baran@company.io', '+48 600 200 229'),
            ('Iwona', 'Duda', 'Utrzymanie ruchu', 'iwona.duda@company.io', '+48 600 200 230'),
            ('Aleksandra', 'Kubiak', 'Rekrutacja', 'aleksandra.kubiak@company.io', '+48 600 200 231'),
            ('Bożena', 'Sadowska', 'Rekrutacja', 'bozena.sadowska@company.io', '+48 600 200 232'),
            ('Celina', 'Mróz', 'Rekrutacja', 'celina.mroz@company.io', NULL),
            ('Danuta', 'Czerwińska', 'Rekrutacja', 'danuta.czerwinska@company.io', '+48 600 200 234'),
            ('Elżbieta', 'Zawadzka', 'Rekrutacja', 'elzbieta.zawadzka@company.io', '+48 600 200 235'),
            ('Franciszka', 'Kalinowska', 'Rekrutacja', 'franciszka.kalinowska@company.io', NULL),
            ('Hanna', 'Wysocka', 'Rekrutacja', 'hanna.wysocka@company.io', '+48 600 200 237'),
            ('Izabela', 'Kozłowska', 'Rekrutacja', 'izabela.kozlowska@company.io', '+48 600 200 238'),
            ('Justyna', 'Sobczak', 'Rekrutacja', 'justyna.sobczak@company.io', NULL),
            ('Karolina', 'Wilk', 'Rekrutacja', 'karolina.wilk@company.io', '+48 600 200 240')) AS p (first_name, last_name, department, email, phone)
    WHERE NOT EXISTS (SELECT 1 FROM crm.people AS x WHERE lower(x.email) = lower(p.email));

    -- Tickets only into an empty desk; one seed at a time.
    LOCK TABLE crm.tickets IN EXCLUSIVE MODE;
    IF p_tickets = 0 OR EXISTS (SELECT 1 FROM crm.tickets) THEN
        RETURN;
    END IF;

    SELECT array_agg(u.id ORDER BY u.login) INTO v_agents
    FROM crm.users AS u
    WHERE u.login IN ('anna.nowak', 'piotr.kowalski', 'katarzyna.wisniewska', 'tomasz.wojcik', 'magdalena.kaminska',
                      'michal.lewandowski', 'agnieszka.zielinska', 'pawel.szymanski', 'joanna.wozniak',
                      'krzysztof.dabrowski');
    -- Generated tickets carry no report data, so only topics without required fields (as Halcyon's seed).
    SELECT array_agg(s.subcategory_key ORDER BY c.sort_order, s.sort_order),
           array_agg(s.category_key ORDER BY c.sort_order, s.sort_order),
           array_agg(s.name ORDER BY c.sort_order, s.sort_order)
    INTO v_subs, v_cats, v_names
    FROM crm.desk_subcategories AS s
    JOIN crm.desk_categories AS c ON c.category_key = s.category_key
    WHERE s.is_active
      AND NOT EXISTS (SELECT 1 FROM crm.desk_subcategory_fields AS sf
                      WHERE sf.subcategory_key = s.subcategory_key AND sf.is_required);
    SELECT array_agg(p.id ORDER BY p.id) INTO v_people FROM crm.people AS p;

    FOR i IN 1..p_tickets
        LOOP
            v_h := seed.hash(p_random_seed, 'desk', i);
            v_sub_n := (seed.h(v_h, 0) % cardinality(v_subs))::int + 1;
            v_priority := CASE seed.h(v_h, 1) % 10
                              WHEN 0 THEN 'CRITICAL' WHEN 1 THEN 'HIGH' WHEN 2 THEN 'HIGH'
                              WHEN 3 THEN 'LOW' WHEN 4 THEN 'LOW' ELSE 'MEDIUM' END;
            v_reported := now() - make_interval(mins => (seed.h(v_h, 2) % (28 * 24 * 60))::int);
            v_agent := v_agents[(seed.h(v_h, 3) % cardinality(v_agents))::int + 1];
            v_person := CASE WHEN seed.h(v_h, 4) % 10 = 0 THEN NULL
                             ELSE v_people[(seed.h(v_h, 4) % cardinality(v_people))::int + 1] END;
            v_title := v_names[v_sub_n] || ' (' || i || ')';

            -- Half registered by an agent (dated back), half from the public form (dated now).
            v_ticket_id := crm.desk_ticket_create(
                    CASE WHEN seed.h(v_h, 1) % 2 = 0 THEN v_agent END, v_title,
                    'Zgłoszenie przykładowe nr ' || i || ': ' || v_names[v_sub_n]
                        || '. Szczegóły w załącznikach i komentarzach.',
                    v_cats[v_sub_n], v_subs[v_sub_n], v_priority,
                    CASE WHEN seed.h(v_h, 1) % 2 = 0 THEN v_reported END,
                    v_person, v_person IS NULL, NULL, NULL);

            IF seed.h(v_h, 3) % 5 < 2 THEN
                CALL app.desk_ticket_take(v_agent, v_ticket_id);
                IF seed.h(v_h, 4) % 2 = 0 THEN
                    CALL app.desk_ticket_resolve(v_agent, v_ticket_id);
                END IF;
            END IF;
        END LOOP;
    RAISE NOTICE 'seed: % desk tickets', p_tickets;
END;
$$;

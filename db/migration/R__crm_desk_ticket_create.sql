/*
  Internal (ADR-0007): a new ticket - the first event of its stream (TicketReported) and its row, in one transaction.
  p_actor_id NULL = the public form (channel SELF_SERVICE, reported now); otherwise an agent registers it (channel
  AGENT) and may date it back up to 30 days (a phone call this morning), never more than 5 minutes ahead.
  The requester is a person from crm.people XOR incognito. A new ticket starts in the pool. Returns its id.
  p_fields (JSON object, ADR-0010): the report data of the subcategory, checked by crm.desk_fields_check.
  p_attachments (JSON array, ADR-0009): files already stored, recorded in the same transaction as the report.
*/
DROP FUNCTION IF EXISTS crm.desk_ticket_create(int, text, text, text, text, text, timestamptz, int, boolean);

CREATE OR REPLACE FUNCTION crm.desk_ticket_create(
    p_actor_id            int,
    p_title               text,
    p_description         text,
    p_category_key        text,
    p_subcategory_key     text,
    p_priority            text,
    p_reported_at         timestamptz,
    p_requester_person_id int,
    p_requester_incognito boolean,
    p_fields              jsonb,
    p_attachments         jsonb
)
    RETURNS bigint
    LANGUAGE plpgsql
AS
$$
DECLARE
    v_now            timestamptz(3) := now();
    v_actor_name     text;
    v_sub_active     boolean;
    v_sub_name       text;
    v_sla            int;
    v_requester_name text;
    v_number         text;
    v_channel        text;
    v_ticket_id      bigint;
    v_normalized     jsonb;
BEGIN
    IF p_actor_id IS NOT NULL THEN
        v_actor_name := crm.desk_actor(p_actor_id);
    ELSE
        p_reported_at := v_now;
    END IF;

    p_title := btrim(p_title);
    p_description := btrim(p_description);
    IF p_title IS NULL OR p_title = '' THEN
        RAISE EXCEPTION 'Title is required.' USING ERRCODE = 'U1100';
    END IF;
    IF length(p_title) > 200 THEN
        RAISE EXCEPTION 'Title can have at most 200 characters.' USING ERRCODE = 'U1101';
    END IF;
    IF p_description IS NULL OR p_description = '' THEN
        RAISE EXCEPTION 'Description is required.' USING ERRCODE = 'U1102';
    END IF;
    IF length(p_description) > 4000 THEN
        RAISE EXCEPTION 'Description can have at most 4000 characters.' USING ERRCODE = 'U1103';
    END IF;

    IF NOT EXISTS (SELECT 1 FROM crm.desk_categories AS c WHERE c.category_key = p_category_key) THEN
        RAISE EXCEPTION 'Choose a category from the list.' USING ERRCODE = 'U1104';
    END IF;
    SELECT s.is_active, s.name INTO v_sub_active, v_sub_name
    FROM crm.desk_subcategories AS s
    WHERE s.subcategory_key = p_subcategory_key
      AND s.category_key = p_category_key;
    IF v_sub_active IS NULL THEN
        RAISE EXCEPTION 'Choose a subcategory of the chosen category.' USING ERRCODE = 'U1105';
    END IF;
    IF NOT v_sub_active THEN
        RAISE EXCEPTION 'Subcategory "%" no longer takes new tickets.', v_sub_name USING ERRCODE = 'U1106';
    END IF;

    v_normalized := crm.desk_fields_check(p_subcategory_key, p_fields);

    SELECT p.sla_minutes INTO v_sla FROM crm.desk_priorities AS p WHERE p.priority = p_priority;
    IF v_sla IS NULL THEN
        RAISE EXCEPTION 'Choose a priority.' USING ERRCODE = 'U1107';
    END IF;
    IF p_reported_at IS NULL THEN
        RAISE EXCEPTION 'The report date is required.' USING ERRCODE = 'U1108';
    END IF;
    IF p_reported_at > v_now + interval '5 minutes' THEN
        RAISE EXCEPTION 'The report date cannot be in the future.' USING ERRCODE = 'U1108';
    END IF;
    IF p_reported_at < v_now - interval '30 days' THEN
        RAISE EXCEPTION 'The report date can be at most 30 days back.' USING ERRCODE = 'U1109';
    END IF;

    p_requester_incognito := coalesce(p_requester_incognito, false);
    IF (p_requester_incognito AND p_requester_person_id IS NOT NULL)
        OR (NOT p_requester_incognito AND p_requester_person_id IS NULL) THEN
        RAISE EXCEPTION 'Choose the requester from the list or mark "incognito".' USING ERRCODE = 'U1110';
    END IF;
    IF p_requester_person_id IS NOT NULL THEN
        SELECT pe.first_name || ' ' || pe.last_name INTO v_requester_name
        FROM crm.people AS pe
        WHERE pe.id = p_requester_person_id;
        IF v_requester_name IS NULL THEN
            RAISE EXCEPTION 'Person not found.' USING ERRCODE = 'U1411';
        END IF;
    END IF;

    -- HAL- and 8 random hex digits; a collision (1 in 4 billion) just draws again.
    LOOP
        v_number := 'HAL-' || upper(substr(replace(gen_random_uuid()::text, '-', ''), 1, 8));
        EXIT WHEN NOT EXISTS (SELECT 1 FROM crm.tickets AS t WHERE t.number = v_number);
    END LOOP;

    v_channel := CASE WHEN p_actor_id IS NULL THEN 'SELF_SERVICE' ELSE 'AGENT' END;

    INSERT INTO crm.tickets (number, title, description, category_key, subcategory_key, priority, original_priority,
                             reported_at, deadline, status, requester_incognito, requester_person_id, requester_name,
                             channel, registered_by_user_id, registered_by_name, recorded_at, version, last_change_at)
    VALUES (v_number, p_title, p_description, p_category_key, p_subcategory_key, p_priority, p_priority,
            p_reported_at, p_reported_at + make_interval(mins => v_sla), 'IN_POOL', p_requester_incognito,
            p_requester_person_id, v_requester_name, v_channel, p_actor_id, v_actor_name, v_now, 1, v_now)
    RETURNING id INTO v_ticket_id;

    PERFORM crm.desk_ticket_append(
        v_ticket_id, 0, 'TicketReported', v_now, p_actor_id, v_actor_name,
        jsonb_build_object(
            'number', v_number, 'title', p_title, 'description', p_description, 'category', p_category_key,
            'subcategory', p_subcategory_key, 'priority', p_priority,
            'reportedAt', crm.json_instant(p_reported_at),
            'requester', jsonb_build_object('incognito', p_requester_incognito, 'personId', p_requester_person_id,
                                            'name', v_requester_name),
            'channel', v_channel, 'registeredBy', v_actor_name,
            'fields', CASE WHEN v_normalized <> '{}'::jsonb THEN v_normalized END));

    INSERT INTO crm.ticket_fields (ticket_id, field_key, value)
    SELECT v_ticket_id, e.key, e.value
    FROM jsonb_each_text(v_normalized) AS e;

    IF p_attachments IS NOT NULL THEN
        PERFORM crm.desk_ticket_add_attachments(v_ticket_id, p_actor_id,
                                                coalesce(v_actor_name, v_requester_name, 'incognito'),
                                                p_attachments, v_now, 1);
    END IF;

    RETURN v_ticket_id;
END;
$$;

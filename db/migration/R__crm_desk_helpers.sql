-- Internal helpers of the service desk (ADR-0007). Not executable by the application (schema crm).

-- The acting agent - any active CRM user - and their name as the events remember it.
CREATE OR REPLACE FUNCTION crm.desk_actor(p_actor_id int)
    RETURNS text
    LANGUAGE plpgsql
    STABLE
AS
$$
DECLARE
    v_name text;
BEGIN
    PERFORM crm.actor_require(p_actor_id);
    SELECT u.display_name INTO v_name FROM crm.users AS u WHERE u.id = p_actor_id;
    RETURN v_name;
END;
$$;

-- An instant in event data: UTC with milliseconds and Z, e.g. 2026-11-08T10:15:30.123Z.
CREATE OR REPLACE FUNCTION crm.json_instant(p_at timestamptz)
    RETURNS text
    LANGUAGE sql
    IMMUTABLE
    PARALLEL SAFE
AS
$$
SELECT to_char(p_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"');
$$;

-- {"id": .., "name": ..} of an agent or a person in event data.
CREATE OR REPLACE FUNCTION crm.json_ref(p_id int, p_name text)
    RETURNS jsonb
    LANGUAGE sql
    IMMUTABLE
    PARALLEL SAFE
AS
$$
SELECT jsonb_build_object('id', p_id, 'name', p_name);
$$;

-- The ticket's row under a row lock, inside the caller's transaction. Two writers of one ticket queue up, so the
-- second decides on the state the first one left ("already in progress with ..."). U1410 when there is none.
CREATE OR REPLACE FUNCTION crm.desk_ticket_lock(p_ticket_id bigint)
    RETURNS crm.tickets
    LANGUAGE plpgsql
AS
$$
DECLARE
    v_ticket crm.tickets;
BEGIN
    SELECT t.* INTO v_ticket FROM crm.tickets AS t WHERE t.id = p_ticket_id FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Ticket not found.' USING ERRCODE = 'U1410';
    END IF;
    RETURN v_ticket;
END;
$$;

-- Appends the next event of the stream. (ticket_id, version) is the key, so a version stored by someone else
-- meanwhile is a conflict (U1913), never an overwrite.
CREATE OR REPLACE FUNCTION crm.desk_ticket_append(
    p_ticket_id        bigint,
    p_expected_version int,
    p_event_type       text,
    p_at               timestamptz,
    p_actor_user_id    int,
    p_actor_name       text,
    p_data             jsonb
)
    RETURNS void
    LANGUAGE plpgsql
AS
$$
BEGIN
    INSERT INTO crm.ticket_events (ticket_id, version, event_type, at, actor_user_id, actor_name, data)
    VALUES (p_ticket_id, p_expected_version + 1, p_event_type, p_at, p_actor_user_id, p_actor_name, p_data);
EXCEPTION
    WHEN unique_violation THEN
        RAISE EXCEPTION 'Someone was faster: the ticket has just changed. Refresh and try again.'
            USING ERRCODE = 'U1913';
END;
$$;

-- The rule of every action of the agent working on a ticket.
-- U1912 in the pool, U1911 resolved, U1310..U1313 somebody else's ticket. p_action: HAND_OVER, RETURN, RESOLVE, HANDLER.
CREATE OR REPLACE FUNCTION crm.desk_require_assignee(p_ticket crm.tickets, p_actor_id int, p_action text)
    RETURNS void
    LANGUAGE plpgsql
    IMMUTABLE
AS
$$
BEGIN
    IF p_ticket.status = 'IN_POOL' THEN
        RAISE EXCEPTION 'The ticket is in the pool: take it first.' USING ERRCODE = 'U1912';
    END IF;
    IF p_ticket.status = 'DONE' THEN
        RAISE EXCEPTION 'The ticket is already resolved.' USING ERRCODE = 'U1911';
    END IF;
    IF p_ticket.assignee_user_id IS DISTINCT FROM p_actor_id THEN
        CASE p_action
            WHEN 'HAND_OVER' THEN RAISE EXCEPTION 'Only the agent working on the ticket (%) can hand it over.',
                p_ticket.assignee_name USING ERRCODE = 'U1310';
            WHEN 'RETURN' THEN RAISE EXCEPTION 'Only the agent working on the ticket (%) can return it to the pool.',
                p_ticket.assignee_name USING ERRCODE = 'U1311';
            WHEN 'RESOLVE' THEN RAISE EXCEPTION 'Only the agent working on the ticket (%) can resolve it.',
                p_ticket.assignee_name USING ERRCODE = 'U1312';
            ELSE RAISE EXCEPTION 'Only the agent working on the ticket (%) can name its handler.',
                p_ticket.assignee_name USING ERRCODE = 'U1313';
            END CASE;
    END IF;
END;
$$;

-- A reason of an action: trimmed, empty -> NULL, at most 500 characters (U1111).
CREATE OR REPLACE FUNCTION crm.desk_reason(p_reason text)
    RETURNS text
    LANGUAGE plpgsql
    IMMUTABLE
AS
$$
BEGIN
    p_reason := crm.trim_to_null(p_reason);
    IF length(p_reason) > 500 THEN
        RAISE EXCEPTION 'Reason can have at most 500 characters.' USING ERRCODE = 'U1111';
    END IF;
    RETURN p_reason;
END;
$$;

-- The search text of the ticket lists: a title fragment (ILIKE pattern, wildcards literal) and, when the text is a
-- ticket number, that number in upper case.
CREATE OR REPLACE FUNCTION crm.desk_contains_pattern(p_search text)
    RETURNS text
    LANGUAGE sql
    IMMUTABLE
    PARALLEL SAFE
AS
$$
SELECT '%' || replace(replace(replace(p_search, '\', '\\'), '%', '\%'), '_', '\_') || '%';
$$;

CREATE OR REPLACE FUNCTION crm.desk_number_of(p_search text)
    RETURNS text
    LANGUAGE sql
    IMMUTABLE
    PARALLEL SAFE
AS
$$
SELECT CASE WHEN upper(btrim(p_search)) ~ '^HAL-[0-9A-F]{8}$' THEN upper(btrim(p_search)) END;
$$;

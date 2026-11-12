/*
  Change the priority of an unresolved ticket (any agent). The deadline is recomputed from the REPORT date with the
  new SLA, never from now (ADR-0007). The same priority again is a no-op: no event, the row stays unchanged.
*/
CREATE OR REPLACE PROCEDURE app.desk_ticket_change_priority(p_actor_id int, p_ticket_id bigint, p_priority text,
                                                            p_reason text)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_actor_name text           := crm.desk_actor(p_actor_id);
    v_now        timestamptz(3) := now();
    v_sla        int;
    v_ticket     crm.tickets;
BEGIN
    SELECT p.sla_minutes INTO v_sla FROM crm.desk_priorities AS p WHERE p.priority = p_priority;
    IF v_sla IS NULL THEN
        RAISE EXCEPTION 'Choose a priority.' USING ERRCODE = 'U1107';
    END IF;
    p_reason := crm.desk_reason(p_reason);

    v_ticket := crm.desk_ticket_lock(p_ticket_id);
    IF v_ticket.status = 'DONE' THEN
        RAISE EXCEPTION 'The ticket is already resolved.' USING ERRCODE = 'U1911';
    END IF;
    IF v_ticket.priority = p_priority THEN
        RETURN;
    END IF;

    PERFORM crm.desk_ticket_append(
        p_ticket_id, v_ticket.version, 'TicketPriorityChanged', v_now, p_actor_id, v_actor_name,
        jsonb_build_object('from', v_ticket.priority, 'to', p_priority,
                           'agent', crm.json_ref(p_actor_id, v_actor_name), 'reason', p_reason));

    UPDATE crm.tickets AS t
    SET priority = p_priority, deadline = t.reported_at + make_interval(mins => v_sla),
        version = v_ticket.version + 1, last_change_at = v_now
    WHERE t.id = p_ticket_id;
END;
$$;

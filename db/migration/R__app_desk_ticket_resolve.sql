-- Resolve a ticket. Only the agent working on it. Resolved at or after the deadline = late, kept for good.
CREATE OR REPLACE PROCEDURE app.desk_ticket_resolve(p_actor_id int, p_ticket_id bigint)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_actor_name text           := crm.desk_actor(p_actor_id);
    v_now        timestamptz(3) := now();
    v_ticket     crm.tickets    := crm.desk_ticket_lock(p_ticket_id);
    v_late       boolean;
BEGIN
    PERFORM crm.desk_require_assignee(v_ticket, p_actor_id, 'RESOLVE');
    v_late := v_now >= v_ticket.deadline;

    PERFORM crm.desk_ticket_append(
        p_ticket_id, v_ticket.version, 'TicketResolved', v_now, p_actor_id, v_actor_name,
        jsonb_build_object('agent', crm.json_ref(p_actor_id, v_actor_name), 'late', v_late));

    UPDATE crm.tickets AS t
    SET status = 'DONE', resolved_at = v_now, resolved_late = v_late, version = v_ticket.version + 1,
        last_change_at = v_now
    WHERE t.id = p_ticket_id;
END;
$$;

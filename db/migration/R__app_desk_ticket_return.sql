-- Give a ticket back to the pool, with an optional reason. Only the agent working on it; the deadline stays.
CREATE OR REPLACE PROCEDURE app.desk_ticket_return(p_actor_id int, p_ticket_id bigint, p_reason text)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_actor_name text           := crm.desk_actor(p_actor_id);
    v_now        timestamptz(3) := now();
    v_ticket     crm.tickets;
BEGIN
    p_reason := crm.desk_reason(p_reason);
    v_ticket := crm.desk_ticket_lock(p_ticket_id);
    PERFORM crm.desk_require_assignee(v_ticket, p_actor_id, 'RETURN');

    PERFORM crm.desk_ticket_append(
        p_ticket_id, v_ticket.version, 'TicketReturnedToPool', v_now, p_actor_id, v_actor_name,
        jsonb_build_object('agent', crm.json_ref(p_actor_id, v_actor_name), 'reason', p_reason));

    UPDATE crm.tickets AS t
    SET status = 'IN_POOL', assignee_user_id = NULL, assignee_name = NULL, version = v_ticket.version + 1,
        last_change_at = v_now
    WHERE t.id = p_ticket_id;
END;
$$;

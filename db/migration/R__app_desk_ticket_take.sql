-- Take a ticket from the pool (or one returned to it). Of two agents taking it at once, the second one is told
-- who was faster (U1910): the row lock makes them decide one after the other.
CREATE OR REPLACE PROCEDURE app.desk_ticket_take(p_actor_id int, p_ticket_id bigint)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_actor_name text           := crm.desk_actor(p_actor_id);
    v_now        timestamptz(3) := now();
    v_ticket     crm.tickets    := crm.desk_ticket_lock(p_ticket_id);
BEGIN
    IF v_ticket.status = 'IN_PROGRESS' THEN
        RAISE EXCEPTION 'The ticket is already in progress with %.', v_ticket.assignee_name USING ERRCODE = 'U1910';
    END IF;
    IF v_ticket.status = 'DONE' THEN
        RAISE EXCEPTION 'The ticket is already resolved.' USING ERRCODE = 'U1911';
    END IF;

    PERFORM crm.desk_ticket_append(p_ticket_id, v_ticket.version, 'TicketTaken', v_now, p_actor_id, v_actor_name,
                                   jsonb_build_object('agent', crm.json_ref(p_actor_id, v_actor_name)));

    UPDATE crm.tickets AS t
    SET status = 'IN_PROGRESS', assignee_user_id = p_actor_id, assignee_name = v_actor_name,
        version = v_ticket.version + 1, last_change_at = v_now
    WHERE t.id = p_ticket_id;
END;
$$;

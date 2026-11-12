-- Hand a ticket over to another agent (an active CRM user). Only the agent working on it; the deadline stays.
CREATE OR REPLACE PROCEDURE app.desk_ticket_hand_over(p_actor_id int, p_ticket_id bigint, p_to_user_id int,
                                                      p_reason text)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_actor_name text           := crm.desk_actor(p_actor_id);
    v_now        timestamptz(3) := now();
    v_to_name    text;
    v_ticket     crm.tickets;
BEGIN
    p_reason := crm.desk_reason(p_reason);
    SELECT u.display_name INTO v_to_name FROM crm.users AS u WHERE u.id = p_to_user_id AND u.is_active;
    IF v_to_name IS NULL THEN
        RAISE EXCEPTION 'Agent not found or not active.' USING ERRCODE = 'U1412';
    END IF;

    v_ticket := crm.desk_ticket_lock(p_ticket_id);
    PERFORM crm.desk_require_assignee(v_ticket, p_actor_id, 'HAND_OVER');
    IF p_to_user_id = p_actor_id THEN
        RAISE EXCEPTION 'A ticket cannot be handed over to yourself.' USING ERRCODE = 'U1114';
    END IF;

    PERFORM crm.desk_ticket_append(
        p_ticket_id, v_ticket.version, 'TicketHandedOver', v_now, p_actor_id, v_actor_name,
        jsonb_build_object('from', crm.json_ref(p_actor_id, v_actor_name), 'to', crm.json_ref(p_to_user_id, v_to_name),
                           'reason', p_reason));

    UPDATE crm.tickets AS t
    SET assignee_user_id = p_to_user_id, assignee_name = v_to_name, version = v_ticket.version + 1,
        last_change_at = v_now
    WHERE t.id = p_ticket_id;
END;
$$;

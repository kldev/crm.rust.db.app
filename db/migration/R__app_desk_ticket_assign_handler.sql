/*
  Name the person (not an agent) who does the work, or take them off (p_person_id NULL). Only the agent working on
  the ticket; when the category has a department, only somebody from it (U1115). The same handler is a no-op.
*/
CREATE OR REPLACE PROCEDURE app.desk_ticket_assign_handler(p_actor_id int, p_ticket_id bigint, p_person_id int,
                                                           p_reason text)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_actor_name        text           := crm.desk_actor(p_actor_id);
    v_now               timestamptz(3) := now();
    v_person_name       text;
    v_person_department text;
    v_department        text;
    v_ticket            crm.tickets;
BEGIN
    p_reason := crm.desk_reason(p_reason);
    IF p_person_id IS NOT NULL THEN
        SELECT pe.first_name || ' ' || pe.last_name, pe.department INTO v_person_name, v_person_department
        FROM crm.people AS pe
        WHERE pe.id = p_person_id;
        IF v_person_name IS NULL THEN
            RAISE EXCEPTION 'Person not found.' USING ERRCODE = 'U1411';
        END IF;
    END IF;

    v_ticket := crm.desk_ticket_lock(p_ticket_id);
    PERFORM crm.desk_require_assignee(v_ticket, p_actor_id, 'HANDLER');

    SELECT c.department INTO v_department FROM crm.desk_categories AS c WHERE c.category_key = v_ticket.category_key;
    IF p_person_id IS NOT NULL AND v_department IS NOT NULL AND v_person_department <> v_department THEN
        RAISE EXCEPTION 'Choose a person from the department %.', v_department USING ERRCODE = 'U1115';
    END IF;
    IF v_ticket.handler_person_id IS NOT DISTINCT FROM p_person_id THEN
        RETURN;
    END IF;

    PERFORM crm.desk_ticket_append(
        p_ticket_id, v_ticket.version, 'TicketHandlerAssigned', v_now, p_actor_id, v_actor_name,
        jsonb_build_object('person', CASE WHEN p_person_id IS NOT NULL THEN crm.json_ref(p_person_id, v_person_name) END,
                           'agent', crm.json_ref(p_actor_id, v_actor_name), 'reason', p_reason));

    UPDATE crm.tickets AS t
    SET handler_person_id = p_person_id, handler_name = v_person_name, version = v_ticket.version + 1,
        last_change_at = v_now
    WHERE t.id = p_ticket_id;
END;
$$;

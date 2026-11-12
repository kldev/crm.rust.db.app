-- Campaigns ordered for a ticket: marketing tickets whose "related-ticket" field is this ticket's number.
CREATE OR REPLACE FUNCTION app.desk_ticket_linked_list(p_actor_id int, p_ticket_id bigint)
    RETURNS TABLE
            (
                id                  bigint,
                number              text,
                title               text,
                description         text,
                category_key        text,
                subcategory_key     text,
                priority            text,
                original_priority   text,
                reported_at         timestamptz,
                deadline            timestamptz,
                status              text,
                assignee_user_id    int,
                assignee_name       text,
                handler_person_id   int,
                handler_name        text,
                requester_incognito boolean,
                requester_person_id int,
                requester_name      text,
                channel             text,
                registered_by_name  text,
                recorded_at         timestamptz,
                resolved_at         timestamptz,
                resolved_late       boolean,
                comment_count       int,
                attachment_count    int,
                version             int,
                last_change_at      timestamptz
            )
    LANGUAGE plpgsql
    STABLE
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_number text;
BEGIN
    PERFORM crm.actor_require(p_actor_id);
    SELECT t.number INTO v_number FROM crm.tickets AS t WHERE t.id = p_ticket_id;
    IF v_number IS NULL THEN
        RAISE EXCEPTION 'Ticket not found.' USING ERRCODE = 'U1410';
    END IF;

    RETURN QUERY
        SELECT t.id, t.number, t.title, t.description, t.category_key, t.subcategory_key, t.priority,
               t.original_priority, t.reported_at, t.deadline, t.status, t.assignee_user_id, t.assignee_name,
               t.handler_person_id, t.handler_name, t.requester_incognito, t.requester_person_id, t.requester_name,
               t.channel, t.registered_by_name, t.recorded_at, t.resolved_at, t.resolved_late, t.comment_count,
               t.attachment_count, t.version, t.last_change_at
        FROM crm.ticket_fields AS tf
        JOIN crm.tickets AS t ON t.id = tf.ticket_id
        WHERE tf.field_key = 'related-ticket'
          AND tf.value = v_number
        ORDER BY t.reported_at DESC;
END;
$$;

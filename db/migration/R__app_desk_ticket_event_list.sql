-- The timeline of a ticket: its stream, oldest first. The UI renders each event type from its JSON.
CREATE OR REPLACE FUNCTION app.desk_ticket_event_list(p_actor_id int, p_ticket_id bigint)
    RETURNS TABLE
            (
                version       int,
                event_type    text,
                at            timestamptz,
                actor_user_id int,
                actor_name    text,
                data          text
            )
    LANGUAGE plpgsql
    STABLE
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
BEGIN
    PERFORM crm.actor_require(p_actor_id);
    IF NOT EXISTS (SELECT 1 FROM crm.tickets AS t WHERE t.id = p_ticket_id) THEN
        RAISE EXCEPTION 'Ticket not found.' USING ERRCODE = 'U1410';
    END IF;

    RETURN QUERY
        SELECT e.version, e.event_type, e.at, e.actor_user_id, e.actor_name, e.data::text
        FROM crm.ticket_events AS e
        WHERE e.ticket_id = p_ticket_id
        ORDER BY e.version;
END;
$$;

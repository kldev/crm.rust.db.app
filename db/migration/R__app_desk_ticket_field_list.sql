-- The report data of a ticket: its current values, in the subcategory's order.
CREATE OR REPLACE FUNCTION app.desk_ticket_field_list(p_actor_id int, p_ticket_id bigint)
    RETURNS TABLE (field_key text, value text)
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
        SELECT tf.field_key, tf.value
        FROM crm.ticket_fields AS tf
        JOIN crm.tickets AS t ON t.id = tf.ticket_id
        LEFT JOIN crm.desk_subcategory_fields AS sf
                  ON sf.subcategory_key = t.subcategory_key AND sf.field_key = tf.field_key
        WHERE tf.ticket_id = p_ticket_id
        ORDER BY coalesce(sf.sort_order, 999), tf.field_key;
END;
$$;

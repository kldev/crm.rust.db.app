/*
  Fill in or correct the report data of a ticket (ADR-0010): p_values is the whole new set - a missing or empty value
  clears the field (a required one cannot be cleared). Any agent, any state, like a comment: numbers often arrive
  after the work is done. The event lists only the changed fields with their label at that moment; nothing changed
  stores nothing. U1146 when the subcategory has no report data.
*/
CREATE OR REPLACE PROCEDURE app.desk_ticket_update_fields(p_actor_id int, p_ticket_id bigint, p_values jsonb)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_actor_name text           := crm.desk_actor(p_actor_id);
    v_now        timestamptz(3) := now();
    v_ticket     crm.tickets    := crm.desk_ticket_lock(p_ticket_id);
    v_normalized jsonb;
    v_changes    jsonb;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM crm.desk_subcategory_fields AS sf WHERE sf.subcategory_key = v_ticket.subcategory_key)
    THEN
        RAISE EXCEPTION 'This subcategory has no report data.' USING ERRCODE = 'U1146';
    END IF;

    v_normalized := crm.desk_fields_check(v_ticket.subcategory_key, p_values);

    SELECT jsonb_agg(jsonb_build_object('key', c.field_key, 'label', c.label, 'from', c.old_value, 'to', c.new_value)
                     ORDER BY c.sort_order)
    INTO v_changes
    FROM (SELECT sf.field_key, f.label, sf.sort_order, cur.value AS old_value, v_normalized ->> sf.field_key AS new_value
          FROM crm.desk_subcategory_fields AS sf
          JOIN crm.desk_fields AS f ON f.field_key = sf.field_key
          LEFT JOIN crm.ticket_fields AS cur ON cur.ticket_id = p_ticket_id AND cur.field_key = sf.field_key
          WHERE sf.subcategory_key = v_ticket.subcategory_key) AS c
    WHERE c.old_value IS DISTINCT FROM c.new_value;

    IF v_changes IS NULL THEN
        RETURN;
    END IF;

    PERFORM crm.desk_ticket_append(
        p_ticket_id, v_ticket.version, 'TicketFieldsUpdated', v_now, p_actor_id, v_actor_name,
        jsonb_build_object('changes', v_changes, 'agent', crm.json_ref(p_actor_id, v_actor_name)));

    DELETE FROM crm.ticket_fields AS tf
    WHERE tf.ticket_id = p_ticket_id
      AND tf.field_key IN (SELECT c ->> 'key' FROM jsonb_array_elements(v_changes) AS c);
    INSERT INTO crm.ticket_fields (ticket_id, field_key, value)
    SELECT p_ticket_id, c ->> 'key', c ->> 'to'
    FROM jsonb_array_elements(v_changes) AS c
    WHERE c ->> 'to' IS NOT NULL;

    UPDATE crm.tickets AS t SET version = v_ticket.version + 1, last_change_at = v_now WHERE t.id = p_ticket_id;
END;
$$;

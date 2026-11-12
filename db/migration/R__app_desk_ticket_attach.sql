-- An agent adds files to a ticket (any state). The bytes are already stored; on a refusal the caller deletes them.
CREATE OR REPLACE PROCEDURE app.desk_ticket_attach(p_actor_id int, p_ticket_id bigint, p_attachments jsonb)
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
    PERFORM crm.desk_ticket_add_attachments(p_ticket_id, p_actor_id, v_actor_name, p_attachments, v_now,
                                            v_ticket.version);
END;
$$;

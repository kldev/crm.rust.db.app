-- A comment (any agent, any state, also on a resolved ticket). Comments are events: never edited or deleted.
CREATE OR REPLACE PROCEDURE app.desk_ticket_comment(p_actor_id int, p_ticket_id bigint, p_text text)
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
    p_text := btrim(p_text);
    IF p_text IS NULL OR p_text = '' THEN
        RAISE EXCEPTION 'Comment is required.' USING ERRCODE = 'U1112';
    END IF;
    IF length(p_text) > 2000 THEN
        RAISE EXCEPTION 'Comment can have at most 2000 characters.' USING ERRCODE = 'U1113';
    END IF;

    v_ticket := crm.desk_ticket_lock(p_ticket_id);
    PERFORM crm.desk_ticket_append(
        p_ticket_id, v_ticket.version, 'TicketCommented', v_now, p_actor_id, v_actor_name,
        jsonb_build_object('agent', crm.json_ref(p_actor_id, v_actor_name), 'text', p_text));

    UPDATE crm.tickets AS t
    SET comment_count = t.comment_count + 1, version = v_ticket.version + 1, last_change_at = v_now
    WHERE t.id = p_ticket_id;
END;
$$;

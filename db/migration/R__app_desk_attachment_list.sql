-- The attachments of a ticket, oldest first.
CREATE OR REPLACE FUNCTION app.desk_attachment_list(p_actor_id int, p_ticket_id bigint)
    RETURNS TABLE
            (
                attachment_id uuid,
                file_name     text,
                content_type  text,
                size_bytes    bigint,
                added_by      text,
                at            timestamptz,
                is_inline     boolean
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
        SELECT a.attachment_id, a.file_name, a.content_type, a.size_bytes, a.added_by, a.at,
               coalesce((SELECT bool_or(x.is_inline)
                         FROM crm.desk_attachment_types AS x
                         WHERE x.content_type = a.content_type), false)
        FROM crm.ticket_attachments AS a
        WHERE a.ticket_id = p_ticket_id
        ORDER BY a.at, a.file_name;
END;
$$;

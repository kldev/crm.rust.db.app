-- One attachment for download (signed-in agents only): where its bytes are and how to send them.
CREATE OR REPLACE FUNCTION app.desk_attachment_get(p_actor_id int, p_ticket_id bigint, p_attachment_id uuid)
    RETURNS TABLE
            (
                attachment_id uuid,
                file_name     text,
                content_type  text,
                size_bytes    bigint,
                storage_key   text,
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
    RETURN QUERY
        SELECT a.attachment_id, a.file_name, a.content_type, a.size_bytes, a.storage_key,
               coalesce((SELECT bool_or(x.is_inline)
                         FROM crm.desk_attachment_types AS x
                         WHERE x.content_type = a.content_type), false)
        FROM crm.ticket_attachments AS a
        WHERE a.ticket_id = p_ticket_id
          AND a.attachment_id = p_attachment_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Attachment not found.' USING ERRCODE = 'U1413';
    END IF;
END;
$$;

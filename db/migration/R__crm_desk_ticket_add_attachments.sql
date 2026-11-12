/*
  Internal (ADR-0009): records stored files as TicketAttachmentAdded events of one ticket and as rows of its read
  model, inside the caller's transaction, after crm.desk_ticket_lock (or the INSERT of a new ticket). p_attachments is
  a JSON array of {id, fileName, size, storageKey}; every file is checked again, and a ticket has at most 10
  attachments (U1124). Allowed in any state, like a comment. p_version is the ticket's version before; returns the
  version after.
*/
CREATE OR REPLACE FUNCTION crm.desk_ticket_add_attachments(
    p_ticket_id   bigint,
    p_actor_id    int,
    p_added_by    text,
    p_attachments jsonb,
    p_at          timestamptz,
    p_version     int
)
    RETURNS int
    LANGUAGE plpgsql
AS
$$
DECLARE
    v_count        int;
    v_new          int;
    f              record;
    v_name         text;
    v_content_type text;
BEGIN
    IF p_attachments IS NULL OR jsonb_typeof(p_attachments) <> 'array' THEN
        RAISE EXCEPTION 'Attachments must be a JSON array.' USING ERRCODE = 'U0003';
    END IF;

    v_new := jsonb_array_length(p_attachments);
    SELECT t.attachment_count INTO v_count FROM crm.tickets AS t WHERE t.id = p_ticket_id;
    IF v_count + v_new > 10 THEN
        RAISE EXCEPTION 'A ticket can have at most 10 attachments.' USING ERRCODE = 'U1124';
    END IF;

    FOR f IN
        SELECT x.id, x."fileName" AS file_name, x.size, x."storageKey" AS storage_key
        FROM jsonb_to_recordset(p_attachments) AS x (id uuid, "fileName" text, size bigint, "storageKey" text)
        LOOP
            SELECT n.p_name, n.p_content_type INTO v_name, v_content_type
            FROM crm.desk_attachment_normalize(f.file_name, f.size) AS n;

            PERFORM crm.desk_ticket_append(
                p_ticket_id, p_version, 'TicketAttachmentAdded', p_at, p_actor_id, p_added_by,
                jsonb_build_object('attachmentId', f.id, 'fileName', v_name, 'contentType', v_content_type,
                                   'size', f.size, 'addedBy', p_added_by,
                                   'agent', CASE WHEN p_actor_id IS NOT NULL THEN crm.json_ref(p_actor_id, p_added_by) END));
            p_version := p_version + 1;

            INSERT INTO crm.ticket_attachments (attachment_id, ticket_id, file_name, content_type, size_bytes,
                                                storage_key, added_by, added_by_user_id, at)
            VALUES (f.id, p_ticket_id, v_name, v_content_type, f.size, f.storage_key, p_added_by, p_actor_id, p_at);
        END LOOP;

    UPDATE crm.tickets AS t
    SET attachment_count = t.attachment_count + v_new, version = p_version, last_change_at = p_at
    WHERE t.id = p_ticket_id;

    RETURN p_version;
END;
$$;

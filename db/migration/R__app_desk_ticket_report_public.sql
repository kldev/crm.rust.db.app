/*
  The public form (no sign-in): a person reports a ticket for themselves or incognito; reported now.
  Picking a name is not authentication (ADR-0007) - the form only creates tickets, so it gets back just the number
  and the deadline (OUT parameters), never the ticket.
  p_attachments: the requester's files, already stored, recorded with the report in one transaction (ADR-0009).
*/
DROP PROCEDURE IF EXISTS app.desk_ticket_report_public(text, text, text, text, text, int, boolean);

CREATE OR REPLACE PROCEDURE app.desk_ticket_report_public(
    p_title               text,
    p_description         text,
    p_category_key        text,
    p_subcategory_key     text,
    p_priority            text,
    p_requester_person_id int,
    p_requester_incognito boolean,
    p_fields              jsonb,
    p_attachments         jsonb,
    OUT p_number          text,
    OUT p_deadline        timestamptz
)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_ticket_id bigint;
BEGIN
    v_ticket_id := crm.desk_ticket_create(NULL, p_title, p_description, p_category_key, p_subcategory_key,
                                          p_priority, NULL, p_requester_person_id, p_requester_incognito,
                                          p_fields, p_attachments);
    SELECT t.number, t.deadline INTO p_number, p_deadline FROM crm.tickets AS t WHERE t.id = v_ticket_id;
END;
$$;

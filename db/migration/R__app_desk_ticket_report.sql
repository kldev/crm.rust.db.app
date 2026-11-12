-- An agent registers a ticket (a call, an e-mail); may date it back up to 30 days. p_ticket_id: the new ticket.
-- p_fields: the report data of the subcategory (ADR-0010).
DROP PROCEDURE IF EXISTS app.desk_ticket_report(int, text, text, text, text, text, timestamptz, int, boolean);

CREATE OR REPLACE PROCEDURE app.desk_ticket_report(
    p_actor_id            int,
    p_title               text,
    p_description         text,
    p_category_key        text,
    p_subcategory_key     text,
    p_priority            text,
    p_reported_at         timestamptz,
    p_requester_person_id int,
    p_requester_incognito boolean,
    p_fields              jsonb,
    OUT p_ticket_id       bigint
)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
BEGIN
    IF p_actor_id IS NULL THEN
        RAISE EXCEPTION 'Actor is not set.' USING ERRCODE = 'U0001';
    END IF;
    p_ticket_id := crm.desk_ticket_create(p_actor_id, p_title, p_description, p_category_key, p_subcategory_key,
                                          p_priority, p_reported_at, p_requester_person_id, p_requester_incognito,
                                          p_fields, NULL);
END;
$$;

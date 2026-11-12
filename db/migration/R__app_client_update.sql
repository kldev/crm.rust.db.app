/*
  Full update of a client with optimistic concurrency: p_row_version must be the value the caller read
  (U1901 when someone else changed the client in the meantime). Identical values = no write, same rowVersion.
*/
CREATE OR REPLACE PROCEDURE app.client_update(
    p_actor_id         int,
    p_id               bigint,
    p_name             text,
    p_nip              text,
    p_email            text,
    p_phone            text,
    p_city             text,
    p_status           text,
    p_assigned_user_id int,
    p_note             text,
    p_row_version      text
)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_actor_role text := crm.actor_require(p_actor_id);
    v_owner_id   int  := CASE WHEN v_actor_role = 'SALES' THEN p_actor_id END;
    v_current    text;
BEGIN
    p_name := crm.trim_to_null(p_name);
    p_nip := crm.client_nip_normalize(p_nip);
    p_email := lower(crm.trim_to_null(p_email));
    p_phone := crm.trim_to_null(p_phone);
    p_city := crm.trim_to_null(p_city);
    p_note := crm.trim_to_null(p_note);

    PERFORM crm.client_validate(p_name, p_nip, p_email, p_status);

    SELECT crm.row_version(c.xmin) INTO v_current
    FROM crm.clients AS c
    WHERE c.id = p_id
      AND (v_owner_id IS NULL OR c.assigned_user_id = v_owner_id)
        FOR UPDATE;

    IF v_current IS NULL THEN
        RAISE EXCEPTION 'Client not found.' USING ERRCODE = 'U1401';
    END IF;
    IF v_current IS DISTINCT FROM lower(p_row_version) THEN
        RAISE EXCEPTION 'The client was changed by someone else. Reload it and try again.' USING ERRCODE = 'U1901';
    END IF;

    PERFORM crm.client_check_assignee(p_actor_id, v_actor_role, p_assigned_user_id);

    UPDATE crm.clients AS c
    SET name             = p_name,
        nip              = p_nip,
        email            = p_email,
        phone            = p_phone,
        city             = p_city,
        status           = p_status,
        assigned_user_id = p_assigned_user_id,
        note             = p_note,
        updated_at       = now(),
        updated_by       = p_actor_id
    WHERE c.id = p_id
      AND (c.name, c.nip, c.email, c.phone, c.city, c.status, c.assigned_user_id, c.note)
        IS DISTINCT FROM (p_name, p_nip, p_email, p_phone, p_city, p_status, p_assigned_user_id, p_note);
END;
$$;

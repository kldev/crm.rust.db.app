-- SALES creates clients for themselves (p_assigned_user_id NULL = the actor); MANAGER/ADMIN assign to a SALES
-- user or leave the client unassigned. A duplicate NIP fails on ux_clients_nip (23505) and becomes 409.
CREATE OR REPLACE PROCEDURE app.client_create(
    p_actor_id         int,
    p_name             text,
    p_nip              text,
    p_email            text,
    p_phone            text,
    p_city             text,
    p_status           text,
    p_assigned_user_id int,
    p_note             text,
    OUT p_id           bigint
)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_actor_role text := crm.actor_require(p_actor_id);
BEGIN
    p_name := crm.trim_to_null(p_name);
    p_nip := crm.client_nip_normalize(p_nip);
    p_email := lower(crm.trim_to_null(p_email));
    p_phone := crm.trim_to_null(p_phone);
    p_city := crm.trim_to_null(p_city);
    p_note := crm.trim_to_null(p_note);
    IF v_actor_role = 'SALES' AND p_assigned_user_id IS NULL THEN
        p_assigned_user_id := p_actor_id;
    END IF;

    PERFORM crm.client_validate(p_name, p_nip, p_email, p_status);
    PERFORM crm.client_check_assignee(p_actor_id, v_actor_role, p_assigned_user_id);

    INSERT INTO crm.clients (name, nip, email, phone, city, status, assigned_user_id, note, created_by, updated_by)
    VALUES (p_name, p_nip, p_email, p_phone, p_city, p_status, p_assigned_user_id, p_note, p_actor_id, p_actor_id)
    RETURNING id INTO p_id;
END;
$$;

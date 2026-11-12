CREATE OR REPLACE FUNCTION app.client_get(p_actor_id int, p_id bigint)
    RETURNS TABLE
            (
                id                  bigint,
                name                text,
                nip                 text,
                email               text,
                phone               text,
                city                text,
                status              text,
                assigned_user_id    int,
                assigned_user_login text,
                note                text,
                created_at          timestamptz,
                created_by          int,
                updated_at          timestamptz,
                updated_by          int,
                row_version         text
            )
    LANGUAGE plpgsql
    STABLE
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_actor_role text := crm.actor_require(p_actor_id);
    v_owner_id   int  := CASE WHEN v_actor_role = 'SALES' THEN p_actor_id END;
BEGIN
    -- A client of another sales user is "not found", not "forbidden": its existence is not revealed.
    RETURN QUERY
        SELECT c.id, c.name, c.nip, c.email, c.phone, c.city, c.status, c.assigned_user_id, u.login, c.note,
               c.created_at, c.created_by, c.updated_at, c.updated_by, crm.row_version(c.xmin)
        FROM crm.clients AS c
        LEFT JOIN crm.users AS u ON u.id = c.assigned_user_id
        WHERE c.id = p_id
          AND (v_owner_id IS NULL OR c.assigned_user_id = v_owner_id);

    IF NOT FOUND THEN
        RAISE EXCEPTION 'Client not found.' USING ERRCODE = 'U1401';
    END IF;
END;
$$;

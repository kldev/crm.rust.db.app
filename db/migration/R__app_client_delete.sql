-- Only MANAGER and ADMIN delete clients; p_row_version as in app.client_update.
CREATE OR REPLACE PROCEDURE app.client_delete(p_actor_id int, p_id bigint, p_row_version text)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_actor_role text := crm.actor_require(p_actor_id);
    v_current    text;
BEGIN
    IF v_actor_role NOT IN ('ADMIN', 'MANAGER') THEN
        RAISE EXCEPTION 'You are not allowed to perform this operation.' USING ERRCODE = 'U1301';
    END IF;

    SELECT crm.row_version(c.xmin) INTO v_current FROM crm.clients AS c WHERE c.id = p_id FOR UPDATE;

    IF v_current IS NULL THEN
        RAISE EXCEPTION 'Client not found.' USING ERRCODE = 'U1401';
    END IF;
    IF v_current IS DISTINCT FROM lower(p_row_version) THEN
        RAISE EXCEPTION 'The client was changed by someone else. Reload it and try again.' USING ERRCODE = 'U1901';
    END IF;
    IF EXISTS (SELECT 1 FROM crm.projects AS p WHERE p.client_id = p_id) THEN
        RAISE EXCEPTION 'The client has projects and cannot be deleted. Set its status to INACTIVE instead.'
            USING ERRCODE = 'U1904';
    END IF;

    DELETE FROM crm.clients AS c WHERE c.id = p_id;
END;
$$;

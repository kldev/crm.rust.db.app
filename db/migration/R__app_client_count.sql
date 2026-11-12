/*
  Number of clients matching the filters of app.client_search (same visibility: SALES counts only own clients).
  Not part of the list: a count reads every matching row, so the list stays keyset-only and the UI calls this
  only when the user asks for the total.
*/
CREATE OR REPLACE FUNCTION app.client_count(
    p_actor_id    int,
    p_name_prefix text,
    p_nip         text,
    p_email       text,
    p_city        text,
    p_status      text
)
    RETURNS TABLE (total bigint)
    LANGUAGE plpgsql
    STABLE
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
    SET plan_cache_mode = force_custom_plan
AS
$$
DECLARE
    v_actor_role text := crm.actor_require(p_actor_id);
    v_owner_id   int  := CASE WHEN v_actor_role = 'SALES' THEN p_actor_id END;
    v_pattern    text;
BEGIN
    IF p_status IS NOT NULL AND p_status NOT IN ('LEAD', 'ACTIVE', 'INACTIVE') THEN
        RAISE EXCEPTION 'Status must be LEAD, ACTIVE or INACTIVE.' USING ERRCODE = 'U1013';
    END IF;

    p_name_prefix := crm.trim_to_null(p_name_prefix);
    v_pattern := CASE WHEN p_name_prefix IS NOT NULL THEN crm.prefix_pattern(p_name_prefix) END;
    p_nip := crm.client_nip_normalize(p_nip);
    p_email := lower(crm.trim_to_null(p_email));
    p_city := crm.trim_to_null(p_city);

    RETURN QUERY
        SELECT count(*)
        FROM crm.clients AS c
        WHERE (v_pattern IS NULL OR (lower(c.name) COLLATE "C") LIKE v_pattern)
          AND (v_owner_id IS NULL OR c.assigned_user_id = v_owner_id)
          AND (p_nip IS NULL OR c.nip = p_nip)
          AND (p_email IS NULL OR c.email = p_email)
          AND (p_city IS NULL OR c.city = p_city)
          AND (p_status IS NULL OR c.status = p_status);
END;
$$;

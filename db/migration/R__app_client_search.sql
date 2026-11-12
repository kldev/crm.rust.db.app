/*
  Clients page with filters. Keyset pagination (no OFFSET: page 100 000 costs the same as page 1):
    - p_name_prefix given: ordered by (lower(name), id), next page after (p_after_name, p_after_id);
    - otherwise:           newest first (id DESC),         next page after p_after_id.
  SALES sees only clients assigned to them. Returns p_page_size + 1 rows so the caller knows if there is a next
  page. plan_cache_mode = force_custom_plan: optional filters get a plan for the values actually passed.
*/
CREATE OR REPLACE FUNCTION app.client_search(
    p_actor_id    int,
    p_name_prefix text,
    p_nip         text,
    p_email       text,
    p_city        text,
    p_status      text,
    p_after_id    bigint,
    p_after_name  text,
    p_page_size   int
)
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
                created_at          timestamptz,
                updated_at          timestamptz,
                row_version         text
            )
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
    v_after_key  text := lower(p_after_name) COLLATE "C";
BEGIN
    IF p_page_size IS NULL OR p_page_size NOT BETWEEN 1 AND 200 THEN
        RAISE EXCEPTION 'Page size must be between 1 and 200.' USING ERRCODE = 'U1014';
    END IF;
    IF p_status IS NOT NULL AND p_status NOT IN ('LEAD', 'ACTIVE', 'INACTIVE') THEN
        RAISE EXCEPTION 'Status must be LEAD, ACTIVE or INACTIVE.' USING ERRCODE = 'U1013';
    END IF;

    p_name_prefix := crm.trim_to_null(p_name_prefix);
    p_nip := crm.client_nip_normalize(p_nip);
    p_email := lower(crm.trim_to_null(p_email));
    p_city := crm.trim_to_null(p_city);

    IF p_name_prefix IS NOT NULL THEN
        v_pattern := crm.prefix_pattern(p_name_prefix);
        RETURN QUERY
            SELECT c.id, c.name, c.nip, c.email, c.phone, c.city, c.status, c.assigned_user_id, u.login,
                   c.created_at, c.updated_at, crm.row_version(c.xmin)
            FROM crm.clients AS c
            LEFT JOIN crm.users AS u ON u.id = c.assigned_user_id
            WHERE (lower(c.name) COLLATE "C") LIKE v_pattern
              AND (v_owner_id IS NULL OR c.assigned_user_id = v_owner_id)
              AND (p_nip IS NULL OR c.nip = p_nip)
              AND (p_email IS NULL OR c.email = p_email)
              AND (p_city IS NULL OR c.city = p_city)
              AND (p_status IS NULL OR c.status = p_status)
              AND (v_after_key IS NULL OR ((lower(c.name) COLLATE "C"), c.id) > (v_after_key, p_after_id))
            ORDER BY (lower(c.name) COLLATE "C"), c.id
            LIMIT p_page_size + 1;
        RETURN;
    END IF;

    RETURN QUERY
        SELECT c.id, c.name, c.nip, c.email, c.phone, c.city, c.status, c.assigned_user_id, u.login,
               c.created_at, c.updated_at, crm.row_version(c.xmin)
        FROM crm.clients AS c
        LEFT JOIN crm.users AS u ON u.id = c.assigned_user_id
        WHERE (v_owner_id IS NULL OR c.assigned_user_id = v_owner_id)
          AND (p_nip IS NULL OR c.nip = p_nip)
          AND (p_email IS NULL OR c.email = p_email)
          AND (p_city IS NULL OR c.city = p_city)
          AND (p_status IS NULL OR c.status = p_status)
          AND (p_after_id IS NULL OR c.id < p_after_id)
        ORDER BY c.id DESC
        LIMIT p_page_size + 1;
END;
$$;

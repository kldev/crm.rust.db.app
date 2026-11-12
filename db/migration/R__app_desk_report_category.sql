/*
  A category report (ADR-0010): the tickets of one category reported between p_from and p_to - whole calendar days in
  Warsaw time, both ends included - with their report data as a jsonb object {field: value}; filters: market,
  subcategory, open / resolved. Ordered by the report date. The caller pivots the fields into columns.
*/
CREATE OR REPLACE FUNCTION app.desk_report_category(
    p_actor_id int,
    p_category_key text,
    p_from date,
    p_to date,
    p_market text,
    p_subcategory_key text,
    p_completion text
)
    RETURNS TABLE
            (
                id                  bigint,
                number              text,
                reported_at         timestamptz,
                market              text,
                subcategory_key     text,
                title               text,
                requester_incognito boolean,
                requester_name      text,
                status              text,
                priority            text,
                assignee_name       text,
                handler_name        text,
                deadline            timestamptz,
                resolved_at         timestamptz,
                resolved_late       boolean,
                fields              jsonb
            )
    LANGUAGE plpgsql
    STABLE
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    -- Warsaw midnights as instants: a DST change moves them, the days stay whole.
    v_from  timestamptz := p_from::timestamp AT TIME ZONE 'Europe/Warsaw';
    v_until timestamptz := (p_to + 1)::timestamp AT TIME ZONE 'Europe/Warsaw';
BEGIN
    PERFORM crm.actor_require(p_actor_id);
    IF NOT EXISTS (SELECT 1 FROM crm.desk_categories AS c WHERE c.category_key = p_category_key) THEN
        RAISE EXCEPTION 'Choose a category from the list.' USING ERRCODE = 'U1104';
    END IF;
    IF p_from IS NULL OR p_to IS NULL OR p_to < p_from OR p_to - p_from > 366 THEN
        RAISE EXCEPTION 'Choose a period of at most one year (from <= to).' USING ERRCODE = 'U1150';
    END IF;
    IF p_completion IS NOT NULL AND p_completion NOT IN ('OPEN', 'DONE') THEN
        RAISE EXCEPTION 'Completion filter must be OPEN or DONE.' USING ERRCODE = 'U1116';
    END IF;

    RETURN QUERY
        SELECT t.id, t.number, t.reported_at, s.market, t.subcategory_key, t.title, t.requester_incognito,
               t.requester_name, t.status, t.priority, t.assignee_name, t.handler_name, t.deadline, t.resolved_at,
               t.resolved_late,
               (SELECT jsonb_object_agg(tf.field_key, tf.value) FROM crm.ticket_fields AS tf
                WHERE tf.ticket_id = t.id)
        FROM crm.tickets AS t
        JOIN crm.desk_subcategories AS s ON s.subcategory_key = t.subcategory_key
        WHERE t.category_key = p_category_key
          AND t.reported_at >= v_from AND t.reported_at < v_until
          AND (p_market IS NULL OR s.market = p_market)
          AND (p_subcategory_key IS NULL OR t.subcategory_key = p_subcategory_key)
          AND (p_completion IS NULL OR (p_completion = 'DONE' AND t.status = 'DONE')
               OR (p_completion = 'OPEN' AND t.status <> 'DONE'))
        ORDER BY t.reported_at, t.id;
END;
$$;

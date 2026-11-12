/*
  The tickets behind the desk charts (ADR-0011): every ticket open at some moment of [p_from, p_to] (Warsaw days, both
  included) or reported in it, optionally of one category. The deadline state of open tickets is computed here, with
  the same clock and the same 60-minute rule as the lists; the Warsaw weekday (0 = Monday) and hour of the report feed
  the heatmap. `now_utc` is that clock, so the caller's "open now" agrees with the lists.
*/
CREATE OR REPLACE FUNCTION app.desk_chart_tickets(p_actor_id int, p_from date, p_to date, p_category_key text)
    RETURNS TABLE
            (
                id               bigint,
                category_key     text,
                subcategory_key  text,
                priority         text,
                status           text,
                reported_at      timestamptz,
                deadline         timestamptz,
                resolved_at      timestamptz,
                resolved_late    boolean,
                assignee_user_id int,
                assignee_name    text,
                is_overdue       boolean,
                is_due_soon      boolean,
                report_weekday   int,
                report_hour      int,
                now_utc          timestamptz
            )
    LANGUAGE plpgsql
    STABLE
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_from  timestamptz := p_from::timestamp AT TIME ZONE 'Europe/Warsaw';
    v_until timestamptz := (p_to + 1)::timestamp AT TIME ZONE 'Europe/Warsaw';
BEGIN
    PERFORM crm.actor_require(p_actor_id);
    IF p_from IS NULL OR p_to IS NULL OR p_to < p_from OR p_to - p_from > 366 THEN
        RAISE EXCEPTION 'Choose a period of at most one year (from <= to).' USING ERRCODE = 'U1150';
    END IF;

    RETURN QUERY
        SELECT t.id, t.category_key, t.subcategory_key, t.priority, t.status, t.reported_at, t.deadline,
               t.resolved_at, t.resolved_late, t.assignee_user_id, t.assignee_name,
               t.status <> 'DONE' AND t.deadline <= now(),
               t.status <> 'DONE' AND t.deadline > now() AND t.deadline < now() + interval '60 minutes',
               extract(isodow FROM t.reported_at AT TIME ZONE 'Europe/Warsaw')::int - 1,
               extract(hour FROM t.reported_at AT TIME ZONE 'Europe/Warsaw')::int,
               now()
        FROM crm.tickets AS t
        WHERE t.reported_at < v_until
          AND (t.resolved_at IS NULL OR t.resolved_at >= v_from)
          AND (p_category_key IS NULL OR t.category_key = p_category_key);
END;
$$;

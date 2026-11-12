/*
  Tickets list (ADR-0007). p_scope: POOL (in the pool), MINE (in progress with me), ALL.
  Order: open tickets by deadline (the most urgent first), then resolved ones, newest first. Keyset pagination in
  two phases: (p_after_phase 0, p_after_time = deadline, p_after_id) inside the open part, (1, resolved_at, id)
  inside the resolved part. Returns p_page_size + 1 rows so the caller knows if there is a next page.
  Filters: category, subcategory, priority, p_deadline (OVERDUE: deadline passed; DUE_SOON: under 60 minutes left -
  both only for open tickets), p_completion (OPEN / DONE, ALL scope only), p_search (title contains, or the exact
  number). Time-based states use the database clock, the same clock that stores the times.
*/
CREATE OR REPLACE FUNCTION app.desk_ticket_search(
    p_actor_id        int,
    p_scope           text,
    p_category_key    text,
    p_subcategory_key text,
    p_priority        text,
    p_deadline        text,
    p_completion      text,
    p_search          text,
    p_after_phase     int,
    p_after_time      timestamptz,
    p_after_id        bigint,
    p_page_size       int
)
    RETURNS TABLE
            (
                id                  bigint,
                number              text,
                title               text,
                description         text,
                category_key        text,
                subcategory_key     text,
                priority            text,
                original_priority   text,
                reported_at         timestamptz,
                deadline            timestamptz,
                status              text,
                assignee_user_id    int,
                assignee_name       text,
                handler_person_id   int,
                handler_name        text,
                requester_incognito boolean,
                requester_person_id int,
                requester_name      text,
                channel             text,
                registered_by_name  text,
                recorded_at         timestamptz,
                resolved_at         timestamptz,
                resolved_late       boolean,
                comment_count       int,
                attachment_count    int,
                version             int,
                last_change_at      timestamptz
            )
    LANGUAGE plpgsql
    STABLE
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
    SET plan_cache_mode = force_custom_plan
AS
$$
DECLARE
    v_now       timestamptz := now();
    v_soon      timestamptz := now() + interval '60 minutes';
    v_n         int;
    v_have      int         := 0;
    v_number    text;
    v_pattern   text;
    v_with_open boolean;
    v_with_done boolean;
BEGIN
    PERFORM crm.actor_require(p_actor_id);
    IF p_page_size IS NULL OR p_page_size NOT BETWEEN 1 AND 200 THEN
        RAISE EXCEPTION 'Page size must be between 1 and 200.' USING ERRCODE = 'U1014';
    END IF;
    IF coalesce(p_scope, '') NOT IN ('POOL', 'MINE', 'ALL') THEN
        RAISE EXCEPTION 'Scope must be POOL, MINE or ALL.' USING ERRCODE = 'U1116';
    END IF;
    IF p_deadline IS NOT NULL AND p_deadline NOT IN ('OVERDUE', 'DUE_SOON') THEN
        RAISE EXCEPTION 'Deadline filter must be OVERDUE or DUE_SOON.' USING ERRCODE = 'U1116';
    END IF;
    IF p_completion IS NOT NULL AND p_completion NOT IN ('OPEN', 'DONE') THEN
        RAISE EXCEPTION 'Completion filter must be OPEN or DONE.' USING ERRCODE = 'U1116';
    END IF;

    v_n := p_page_size + 1;
    p_search := crm.trim_to_null(p_search);
    v_number := crm.desk_number_of(p_search);
    v_pattern := crm.desk_contains_pattern(p_search);
    -- The pool and my tickets are open by definition; a deadline filter only makes sense for open tickets.
    v_with_open := NOT (coalesce(p_completion, '') = 'DONE' AND p_scope = 'ALL');
    v_with_done := p_scope = 'ALL' AND coalesce(p_completion, 'DONE') = 'DONE' AND p_deadline IS NULL;

    IF v_with_open AND coalesce(p_after_phase, 0) = 0 THEN
        RETURN QUERY
            SELECT t.id, t.number, t.title, t.description, t.category_key, t.subcategory_key, t.priority,
               t.original_priority, t.reported_at, t.deadline, t.status, t.assignee_user_id, t.assignee_name,
               t.handler_person_id, t.handler_name, t.requester_incognito, t.requester_person_id, t.requester_name,
               t.channel, t.registered_by_name, t.recorded_at, t.resolved_at, t.resolved_late, t.comment_count,
               t.attachment_count, t.version, t.last_change_at
            FROM crm.tickets AS t
            WHERE t.status <> 'DONE'
              AND (p_scope <> 'POOL' OR t.status = 'IN_POOL')
              AND (p_scope <> 'MINE' OR (t.status = 'IN_PROGRESS' AND t.assignee_user_id = p_actor_id))
              AND (p_category_key IS NULL OR t.category_key = p_category_key)
              AND (p_subcategory_key IS NULL OR t.subcategory_key = p_subcategory_key)
              AND (p_priority IS NULL OR t.priority = p_priority)
              AND (p_deadline IS NULL OR (p_deadline = 'OVERDUE' AND t.deadline <= v_now)
                OR (p_deadline = 'DUE_SOON' AND t.deadline > v_now AND t.deadline < v_soon))
              AND (p_search IS NULL OR t.title ILIKE v_pattern OR t.number = v_number)
              AND (p_after_phase IS NULL OR (t.deadline, t.id) > (p_after_time, p_after_id))
            ORDER BY t.deadline, t.id
            LIMIT v_n;
        GET DIAGNOSTICS v_have = ROW_COUNT;
    END IF;

    IF v_with_done AND v_have < v_n THEN
        RETURN QUERY
            SELECT t.id, t.number, t.title, t.description, t.category_key, t.subcategory_key, t.priority,
               t.original_priority, t.reported_at, t.deadline, t.status, t.assignee_user_id, t.assignee_name,
               t.handler_person_id, t.handler_name, t.requester_incognito, t.requester_person_id, t.requester_name,
               t.channel, t.registered_by_name, t.recorded_at, t.resolved_at, t.resolved_late, t.comment_count,
               t.attachment_count, t.version, t.last_change_at
            FROM crm.tickets AS t
            WHERE t.status = 'DONE'
              AND (p_category_key IS NULL OR t.category_key = p_category_key)
              AND (p_subcategory_key IS NULL OR t.subcategory_key = p_subcategory_key)
              AND (p_priority IS NULL OR t.priority = p_priority)
              AND (p_search IS NULL OR t.title ILIKE v_pattern OR t.number = v_number)
              AND (coalesce(p_after_phase, 0) = 0 OR (t.resolved_at, t.id) < (p_after_time, p_after_id))
            ORDER BY t.resolved_at DESC, t.id DESC
            LIMIT v_n - v_have;
    END IF;
END;
$$;

-- Number of tickets matching the filters of app.desk_ticket_search. Called only when the user asks for the total.
CREATE OR REPLACE FUNCTION app.desk_ticket_count(
    p_actor_id        int,
    p_scope           text,
    p_category_key    text,
    p_subcategory_key text,
    p_priority        text,
    p_deadline        text,
    p_completion      text,
    p_search          text
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
    v_now       timestamptz := now();
    v_soon      timestamptz := now() + interval '60 minutes';
    v_number    text;
    v_pattern   text;
    v_with_open boolean;
    v_with_done boolean;
BEGIN
    PERFORM crm.actor_require(p_actor_id);
    IF coalesce(p_scope, '') NOT IN ('POOL', 'MINE', 'ALL') THEN
        RAISE EXCEPTION 'Scope must be POOL, MINE or ALL.' USING ERRCODE = 'U1116';
    END IF;
    IF p_deadline IS NOT NULL AND p_deadline NOT IN ('OVERDUE', 'DUE_SOON') THEN
        RAISE EXCEPTION 'Deadline filter must be OVERDUE or DUE_SOON.' USING ERRCODE = 'U1116';
    END IF;
    IF p_completion IS NOT NULL AND p_completion NOT IN ('OPEN', 'DONE') THEN
        RAISE EXCEPTION 'Completion filter must be OPEN or DONE.' USING ERRCODE = 'U1116';
    END IF;

    p_search := crm.trim_to_null(p_search);
    v_number := crm.desk_number_of(p_search);
    v_pattern := crm.desk_contains_pattern(p_search);
    v_with_open := NOT (coalesce(p_completion, '') = 'DONE' AND p_scope = 'ALL');
    v_with_done := p_scope = 'ALL' AND coalesce(p_completion, 'DONE') = 'DONE' AND p_deadline IS NULL;

    RETURN QUERY
        SELECT count(*)
        FROM crm.tickets AS t
        WHERE ((v_with_open AND t.status <> 'DONE') OR (v_with_done AND t.status = 'DONE'))
          AND (p_scope <> 'POOL' OR t.status = 'IN_POOL')
          AND (p_scope <> 'MINE' OR (t.status = 'IN_PROGRESS' AND t.assignee_user_id = p_actor_id))
          AND (p_category_key IS NULL OR t.category_key = p_category_key)
          AND (p_subcategory_key IS NULL OR t.subcategory_key = p_subcategory_key)
          AND (p_priority IS NULL OR t.priority = p_priority)
          AND (p_deadline IS NULL OR (p_deadline = 'OVERDUE' AND t.deadline <= v_now)
            OR (p_deadline = 'DUE_SOON' AND t.deadline > v_now AND t.deadline < v_soon))
          AND (p_search IS NULL OR t.title ILIKE v_pattern OR t.number = v_number);
END;
$$;

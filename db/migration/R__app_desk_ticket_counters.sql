-- The counters of the top bar: overdue and due soon (open tickets), in the pool, and in progress with me.
CREATE OR REPLACE FUNCTION app.desk_ticket_counters(p_actor_id int)
    RETURNS TABLE (overdue int, due_soon int, pool int, mine int)
    LANGUAGE plpgsql
    STABLE
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
BEGIN
    PERFORM crm.actor_require(p_actor_id);
    RETURN QUERY
        SELECT (count(*) FILTER (WHERE t.deadline <= now()))::int,
               (count(*) FILTER (WHERE t.deadline > now() AND t.deadline < now() + interval '60 minutes'))::int,
               (count(*) FILTER (WHERE t.status = 'IN_POOL'))::int,
               (count(*) FILTER (WHERE t.status = 'IN_PROGRESS' AND t.assignee_user_id = p_actor_id))::int
        FROM crm.tickets AS t
        WHERE t.status <> 'DONE';
END;
$$;

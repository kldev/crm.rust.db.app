/*
  How the agents work (ADR-0010): per agent and period (crm.desk_report_periods), the tickets they took from the pool,
  resolved, and resolved after the deadline - counted from the events, so a ticket handed over counts for whoever
  really took or resolved it - and what each one has in progress now. Agents without any of it are left out.
*/
CREATE OR REPLACE FUNCTION app.desk_report_agents(p_actor_id int, p_previous boolean)
    RETURNS TABLE
            (
                user_id        int,
                display_name   text,
                taken_week     int,
                resolved_week  int,
                late_week      int,
                taken_month    int,
                resolved_month int,
                late_month     int,
                taken_three    int,
                resolved_three int,
                late_three     int,
                in_progress    int
            )
    LANGUAGE plpgsql
    STABLE
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    p  record;
    w0 timestamptz; w1 timestamptz; m0 timestamptz; m1 timestamptz; t0 timestamptz; t1 timestamptz;
BEGIN
    PERFORM crm.actor_require(p_actor_id);
    SELECT * INTO p FROM crm.desk_report_periods(coalesce(p_previous, false));
    w0 := p.week_from::timestamp AT TIME ZONE 'Europe/Warsaw';
    w1 := p.week_to::timestamp AT TIME ZONE 'Europe/Warsaw';
    m0 := p.month_from::timestamp AT TIME ZONE 'Europe/Warsaw';
    m1 := p.month_to::timestamp AT TIME ZONE 'Europe/Warsaw';
    t0 := p.three_from::timestamp AT TIME ZONE 'Europe/Warsaw';
    t1 := p.three_to::timestamp AT TIME ZONE 'Europe/Warsaw';

    RETURN QUERY
        WITH work AS (
            SELECT e.actor_user_id AS uid, e.at,
                   e.event_type = 'TicketTaken' AS taken,
                   e.event_type = 'TicketResolved' AS resolved,
                   e.event_type = 'TicketResolved' AND (e.data ->> 'late')::boolean AS late
            FROM crm.ticket_events AS e
            WHERE e.event_type IN ('TicketTaken', 'TicketResolved')
              AND e.at >= least(w0, m0, t0) AND e.at < greatest(w1, m1, t1)
              AND e.actor_user_id IS NOT NULL
        ),
        per_agent AS (
            SELECT uid,
                   count(*) FILTER (WHERE taken AND at >= w0 AND at < w1)::int    AS taken_week,
                   count(*) FILTER (WHERE resolved AND at >= w0 AND at < w1)::int AS resolved_week,
                   count(*) FILTER (WHERE late AND at >= w0 AND at < w1)::int     AS late_week,
                   count(*) FILTER (WHERE taken AND at >= m0 AND at < m1)::int    AS taken_month,
                   count(*) FILTER (WHERE resolved AND at >= m0 AND at < m1)::int AS resolved_month,
                   count(*) FILTER (WHERE late AND at >= m0 AND at < m1)::int     AS late_month,
                   count(*) FILTER (WHERE taken AND at >= t0 AND at < t1)::int    AS taken_three,
                   count(*) FILTER (WHERE resolved AND at >= t0 AND at < t1)::int AS resolved_three,
                   count(*) FILTER (WHERE late AND at >= t0 AND at < t1)::int     AS late_three
            FROM work
            GROUP BY uid
        ),
        now_open AS (
            SELECT t.assignee_user_id AS uid, count(*)::int AS in_progress
            FROM crm.tickets AS t
            WHERE t.status = 'IN_PROGRESS'
            GROUP BY t.assignee_user_id
        )
        SELECT u.id, u.display_name,
               coalesce(a.taken_week, 0), coalesce(a.resolved_week, 0), coalesce(a.late_week, 0),
               coalesce(a.taken_month, 0), coalesce(a.resolved_month, 0), coalesce(a.late_month, 0),
               coalesce(a.taken_three, 0), coalesce(a.resolved_three, 0), coalesce(a.late_three, 0),
               coalesce(o.in_progress, 0)
        FROM crm.users AS u
        LEFT JOIN per_agent AS a ON a.uid = u.id
        LEFT JOIN now_open AS o ON o.uid = u.id
        WHERE a.uid IS NOT NULL OR o.uid IS NOT NULL
        ORDER BY u.display_name, u.id;
END;
$$;

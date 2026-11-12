-- The periods of the agents' report (local Warsaw days, [from, to)), for the headers also when nobody worked.
CREATE OR REPLACE FUNCTION app.desk_report_period_list(p_actor_id int, p_previous boolean)
    RETURNS TABLE (week_from date, week_to date, month_from date, month_to date, three_from date, three_to date)
    LANGUAGE plpgsql
    STABLE
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
BEGIN
    PERFORM crm.actor_require(p_actor_id);
    RETURN QUERY SELECT * FROM crm.desk_report_periods(coalesce(p_previous, false));
END;
$$;

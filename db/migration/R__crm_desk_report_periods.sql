/*
  Internal (ADR-0010): the calendar periods of the agents' report in Warsaw time - this week (Monday to Sunday), this
  month, and this month with the two before - or, with p_previous, the ones just before them. Local days, [from, to)
  half-open. p_today is for tests; callers leave it to today in Warsaw.
*/
CREATE OR REPLACE FUNCTION crm.desk_report_periods(
    p_previous boolean,
    p_today date DEFAULT (now() AT TIME ZONE 'Europe/Warsaw')::date
)
    RETURNS TABLE (week_from date, week_to date, month_from date, month_to date, three_from date, three_to date)
    LANGUAGE sql
    STABLE
    PARALLEL SAFE
    SET search_path = pg_catalog, pg_temp
AS
$$
WITH base AS (SELECT date_trunc('week', p_today::timestamp)::date AS week,
                    date_trunc('month', p_today::timestamp)::date AS month),
     p AS (SELECT CASE WHEN p_previous THEN week - 7 ELSE week END                                  AS wf,
                  CASE WHEN p_previous THEN (month - interval '1 month')::date ELSE month END        AS mf,
                  CASE WHEN p_previous THEN (month - interval '5 months')::date
                       ELSE (month - interval '2 months')::date END                                  AS tf,
                  CASE WHEN p_previous THEN (month - interval '2 months')::date
                       ELSE (month + interval '1 month')::date END                                   AS tt
           FROM base)
SELECT wf, wf + 7, mf, (mf + interval '1 month')::date, tf, tt
FROM p;
$$;

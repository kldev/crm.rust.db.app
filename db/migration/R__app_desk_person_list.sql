-- People (requesters and handlers) with their contacts, for agents. Small list, by surname.
CREATE OR REPLACE FUNCTION app.desk_person_list(p_actor_id int)
    RETURNS TABLE
            (
                id         int,
                first_name text,
                last_name  text,
                department text,
                email      text,
                phone      text
            )
    LANGUAGE plpgsql
    STABLE
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
BEGIN
    PERFORM crm.actor_require(p_actor_id);
    RETURN QUERY
        SELECT pe.id, pe.first_name, pe.last_name, pe.department, pe.email, pe.phone
        FROM crm.people AS pe
        ORDER BY pe.last_name, pe.first_name, pe.id;
END;
$$;

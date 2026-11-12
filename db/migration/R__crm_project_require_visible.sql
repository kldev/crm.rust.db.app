-- Internal helper: can the actor see (and work on) the project? MANAGER and ADMIN see all projects; SALES sees
-- projects they own, projects of their teams and projects of clients assigned to them. Otherwise U1403 - the
-- project's existence is not revealed.
CREATE OR REPLACE FUNCTION crm.project_require_visible(p_actor_id int, p_actor_role text, p_project_id bigint)
    RETURNS void
    LANGUAGE plpgsql
    STABLE
AS
$$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM crm.projects AS p
        WHERE p.id = p_project_id
          AND (p_actor_role IN ('ADMIN', 'MANAGER')
            OR p.owner_user_id = p_actor_id
            OR EXISTS (SELECT 1 FROM crm.team_members AS tm WHERE tm.team_id = p.team_id AND tm.user_id = p_actor_id)
            OR EXISTS (SELECT 1 FROM crm.clients AS c WHERE c.id = p.client_id AND c.assigned_user_id = p_actor_id)))
    THEN
        RAISE EXCEPTION 'Project not found.' USING ERRCODE = 'U1403';
    END IF;
END;
$$;

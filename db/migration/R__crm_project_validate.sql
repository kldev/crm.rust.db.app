-- Internal helper: project field rules and the owner/team rule, shared by create and update.
CREATE OR REPLACE FUNCTION crm.project_validate(
    p_name          text,
    p_team_id       int,
    p_owner_user_id int,
    p_start_date    date,
    p_due_date      date,
    p_budget        numeric
)
    RETURNS void
    LANGUAGE plpgsql
    STABLE
AS
$$
BEGIN
    IF p_name IS NULL OR length(p_name) > 200 THEN
        RAISE EXCEPTION 'Project name is required.' USING ERRCODE = 'U1030';
    END IF;
    IF p_start_date IS NULL OR p_due_date IS NULL OR p_due_date < p_start_date THEN
        RAISE EXCEPTION 'Start and due dates are required and the due date cannot be before the start date.'
            USING ERRCODE = 'U1032';
    END IF;
    IF p_budget IS NULL OR p_budget < 0 OR p_budget >= 1e17 OR p_budget <> round(p_budget, 2) THEN
        RAISE EXCEPTION 'Budget must be 0 or more.' USING ERRCODE = 'U1033';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM crm.teams AS t WHERE t.id = p_team_id) THEN
        RAISE EXCEPTION 'Team not found.' USING ERRCODE = 'U1404';
    END IF;
    IF NOT EXISTS (SELECT 1
                   FROM crm.team_members AS tm
                   JOIN crm.users AS u ON u.id = tm.user_id
                   WHERE tm.team_id = p_team_id
                     AND tm.user_id = p_owner_user_id
                     AND u.is_active) THEN
        RAISE EXCEPTION 'The project owner must be an active member of the project team.' USING ERRCODE = 'U1037';
    END IF;
END;
$$;

-- Internal helper: who may own a client. SALES works only on own clients; MANAGER and ADMIN assign a client
-- to an active SALES user or leave it unassigned.
CREATE OR REPLACE FUNCTION crm.client_check_assignee(p_actor_id int, p_actor_role text, p_assigned_user_id int)
    RETURNS void
    LANGUAGE plpgsql
    STABLE
AS
$$
BEGIN
    IF p_actor_role = 'SALES' THEN
        IF p_assigned_user_id IS DISTINCT FROM p_actor_id THEN
            RAISE EXCEPTION 'A sales user can only work on clients assigned to themselves.' USING ERRCODE = 'U1303';
        END IF;
        RETURN;
    END IF;

    IF p_assigned_user_id IS NOT NULL
        AND NOT EXISTS (SELECT 1
                        FROM crm.users u
                        WHERE u.id = p_assigned_user_id
                          AND u.role = 'SALES'
                          AND u.is_active) THEN
        RAISE EXCEPTION 'A client can be assigned only to an active sales user.' USING ERRCODE = 'U1017';
    END IF;
END;
$$;

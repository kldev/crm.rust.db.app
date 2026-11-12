-- Internal helper: the role of the acting user. Not executable by the application (schema crm).
-- U0001 (50001) = the host did not pass an actor (a bug), U1302 (51302) = the user is unknown or deactivated.
CREATE OR REPLACE FUNCTION crm.actor_require(p_actor_id int)
    RETURNS text
    LANGUAGE plpgsql
    STABLE
AS
$$
DECLARE
    v_role text;
BEGIN
    IF p_actor_id IS NULL THEN
        RAISE EXCEPTION 'Actor is not set.' USING ERRCODE = 'U0001';
    END IF;

    SELECT u.role INTO v_role
    FROM crm.users u
    WHERE u.id = p_actor_id
      AND u.is_active;

    IF v_role IS NULL THEN
        RAISE EXCEPTION 'Your account is not active.' USING ERRCODE = 'U1302';
    END IF;

    RETURN v_role;
END;
$$;

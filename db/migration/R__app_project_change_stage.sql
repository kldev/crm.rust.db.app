/*
  Moves a project to another stage. Allowed moves are rows of crm.project_stage_transitions (otherwise U1902).
  Writes the history row, the team statistics and, when given, a comment - in one transaction with the change.
  Same stage = no change: no history row, no comment, same rowVersion.
*/
CREATE OR REPLACE PROCEDURE app.project_change_stage(
    p_actor_id    int,
    p_id          bigint,
    p_to_stage    text,
    p_comment     text,
    p_row_version text
)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
DECLARE
    v_actor_role text := crm.actor_require(p_actor_id);
    v_current    record;
BEGIN
    IF p_to_stage IS NULL OR p_to_stage NOT IN ('NEW', 'ACCEPTED', 'ACTIVE', 'ON_HOLD', 'CLOSED', 'DECLINED') THEN
        RAISE EXCEPTION 'Stage must be NEW, ACCEPTED, ACTIVE, ON_HOLD, CLOSED or DECLINED.' USING ERRCODE = 'U1031';
    END IF;

    PERFORM crm.project_require_visible(p_actor_id, v_actor_role, p_id);
    p_comment := crm.trim_to_null(p_comment);

    SELECT p.*, crm.row_version(p.xmin) AS rv INTO v_current FROM crm.projects AS p WHERE p.id = p_id FOR UPDATE;

    IF v_current.rv IS DISTINCT FROM lower(p_row_version) THEN
        RAISE EXCEPTION 'The project was changed by someone else. Reload it and try again.' USING ERRCODE = 'U1901';
    END IF;

    IF v_current.stage = p_to_stage THEN
        RETURN;
    END IF;

    IF NOT EXISTS (SELECT 1
                   FROM crm.project_stage_transitions AS t
                   WHERE t.from_stage = v_current.stage
                     AND t.to_stage = p_to_stage) THEN
        RAISE EXCEPTION 'This stage change is not allowed.' USING ERRCODE = 'U1902';
    END IF;

    UPDATE crm.projects AS p
    SET stage            = p_to_stage,
        stage_changed_at = now(),
        updated_at       = now(),
        updated_by       = p_actor_id
    WHERE p.id = p_id;

    INSERT INTO crm.project_stage_history (project_id, from_stage, to_stage, changed_by)
    VALUES (p_id, v_current.stage, p_to_stage, p_actor_id);

    IF p_comment IS NOT NULL THEN
        INSERT INTO crm.project_comments (project_id, author_id, body) VALUES (p_id, p_actor_id, p_comment);
    END IF;

    PERFORM crm.team_stats_move(v_current.team_id, v_current.stage, v_current.budget,
                                v_current.team_id, p_to_stage, v_current.budget);
END;
$$;

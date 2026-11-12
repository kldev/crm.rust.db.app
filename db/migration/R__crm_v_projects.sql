/*
  Read model of a project: one row with client, team, owner and team lead. Functions read projects only through
  this view. A plain view: the planner folds it into the query, so filters reach the indexes of crm.projects and
  the joins are single-row lookups by primary key. Repeatable (R__) because it uses crm.row_version.
*/
CREATE OR REPLACE VIEW crm.v_projects AS
SELECT p.id,
       p.name,
       p.stage,
       p.start_date,
       p.due_date,
       p.budget,
       (p.stage IN ('NEW', 'ACCEPTED', 'ACTIVE', 'ON_HOLD')
           AND p.due_date < (now() AT TIME ZONE 'UTC')::date) AS is_overdue,
       p.stage_changed_at,
       p.client_id,
       c.name                                                AS client_name,
       c.city                                                AS client_city,
       c.assigned_user_id                                    AS client_owner_id,
       p.team_id,
       t.name                                                AS team_name,
       t.lead_user_id                                        AS team_lead_id,
       p.owner_user_id,
       ou.login                                              AS owner_login,
       ou.display_name                                       AS owner_display_name,
       p.created_at,
       p.updated_at,
       crm.row_version(p.xmin)                               AS row_version
FROM crm.projects AS p
JOIN crm.clients AS c ON c.id = p.client_id
JOIN crm.teams AS t ON t.id = p.team_id
JOIN crm.users AS ou ON ou.id = p.owner_user_id;

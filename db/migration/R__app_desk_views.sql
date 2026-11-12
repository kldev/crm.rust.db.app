-- Dictionaries of the desk that need no actor: plain views of schema app (the application reads them with SELECT;
-- they run with the owner's rights, so the application still has no access to the tables).

-- The catalog for the forms and filters: one row per subcategory with its category, in catalog order. Public.
CREATE OR REPLACE VIEW app.v_desk_catalog AS
SELECT c.category_key, c.name AS category_name, c.department, s.subcategory_key, s.name AS subcategory_name,
       s.market, s.is_active, c.sort_order AS category_order, s.sort_order AS subcategory_order
FROM crm.desk_categories AS c
JOIN crm.desk_subcategories AS s ON s.category_key = c.category_key;

-- Priorities with their SLA in minutes (the form shows the deadline before sending). Public.
CREATE OR REPLACE VIEW app.v_desk_priorities AS
SELECT p.priority, p.sla_minutes, p.sort_order
FROM crm.desk_priorities AS p;

-- People for the public form: names and departments only, never the contacts (the page needs no sign-in).
CREATE OR REPLACE VIEW app.v_desk_public_people AS
SELECT pe.id, pe.first_name, pe.last_name, pe.department
FROM crm.people AS pe;

-- Allowed attachment types, for the file picker's accept attribute. Public.
CREATE OR REPLACE VIEW app.v_desk_attachment_types AS
SELECT t.extension, t.content_type
FROM crm.desk_attachment_types AS t;

-- Report fields of every subcategory, in the form's order, for the catalog. Public (the public form shows them).
CREATE OR REPLACE VIEW app.v_desk_fields AS
SELECT sf.subcategory_key, f.field_key, f.label, f.kind, f.placeholder, sf.is_required, sf.sort_order
FROM crm.desk_subcategory_fields AS sf
JOIN crm.desk_fields AS f ON f.field_key = sf.field_key;

-- The values of choice fields, in list order; favourites are offered first by the form. Public.
CREATE OR REPLACE VIEW app.v_desk_field_options AS
SELECT o.field_key, o.option_value, o.is_favorite, o.sort_order
FROM crm.desk_field_options AS o;

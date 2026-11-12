-- Internal helpers: the normal form of client fields (trimmed, empty -> NULL; NIP without '-' and spaces;
-- e-mail in lower case), used by the writes and by the search filters alike.
CREATE OR REPLACE FUNCTION crm.trim_to_null(p_value text)
    RETURNS text
    LANGUAGE sql
    IMMUTABLE
    PARALLEL SAFE
AS
$$
SELECT nullif(btrim(p_value), '');
$$;

CREATE OR REPLACE FUNCTION crm.client_nip_normalize(p_nip text)
    RETURNS text
    LANGUAGE sql
    IMMUTABLE
    PARALLEL SAFE
AS
$$
SELECT nullif(replace(replace(btrim(p_nip), '-', ''), ' ', ''), '');
$$;

-- A name prefix typed by the user as a LIKE pattern on lower(name): its wildcards are literals.
CREATE OR REPLACE FUNCTION crm.prefix_pattern(p_prefix text)
    RETURNS text
    LANGUAGE sql
    IMMUTABLE
    PARALLEL SAFE
AS
$$
SELECT replace(replace(replace(lower(p_prefix), '\', '\\'), '%', '\%'), '_', '\_') || '%';
$$;

-- ADR-0013: rowVersion in the API is the row's xmin (the transaction that wrote it last) as 16 hex digits.
-- An UPDATE that changes nothing is skipped by the procedures (IS DISTINCT FROM), so xmin stays the same.
CREATE OR REPLACE FUNCTION crm.row_version(p_xmin xid)
    RETURNS text
    LANGUAGE sql
    IMMUTABLE
    PARALLEL SAFE
AS
$$
SELECT lpad(to_hex(p_xmin::text::bigint), 16, '0');
$$;

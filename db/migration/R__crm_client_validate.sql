-- Internal helper: client field rules shared by app.client_create and app.client_update.
-- Expects values already normalised (trimmed, empty -> NULL) by the caller.
CREATE OR REPLACE FUNCTION crm.client_validate(p_name text, p_nip text, p_email text, p_status text)
    RETURNS void
    LANGUAGE plpgsql
    IMMUTABLE
AS
$$
BEGIN
    IF p_name IS NULL OR length(p_name) > 200 THEN
        RAISE EXCEPTION 'Client name is required.' USING ERRCODE = 'U1010';
    END IF;
    IF p_nip IS NOT NULL AND p_nip !~ '^[0-9]{10}$' THEN
        RAISE EXCEPTION 'NIP must have exactly 10 digits.' USING ERRCODE = 'U1011';
    END IF;
    IF p_email IS NOT NULL AND (p_email NOT LIKE '_%@_%._%' OR length(p_email) > 254) THEN
        RAISE EXCEPTION 'E-mail address is not valid.' USING ERRCODE = 'U1012';
    END IF;
    IF p_status IS NULL OR p_status NOT IN ('LEAD', 'ACTIVE', 'INACTIVE') THEN
        RAISE EXCEPTION 'Status must be LEAD, ACTIVE or INACTIVE.' USING ERRCODE = 'U1013';
    END IF;
END;
$$;

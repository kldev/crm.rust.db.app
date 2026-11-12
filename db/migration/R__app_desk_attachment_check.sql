-- Checks one file before its bytes are stored (ADR-0009): returns the name as stored and its content type, or refuses
-- it. The same rules run again when the attachment is recorded; this call only spares an upload that would be refused.
CREATE OR REPLACE FUNCTION app.desk_attachment_check(p_file_name text, p_size_bytes bigint)
    RETURNS TABLE (file_name text, content_type text)
    LANGUAGE sql
    STABLE
    SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp
AS
$$
SELECT n.p_name, n.p_content_type
FROM crm.desk_attachment_normalize(p_file_name, p_size_bytes) AS n;
$$;

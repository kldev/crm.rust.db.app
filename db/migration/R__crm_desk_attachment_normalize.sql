/*
  Internal (ADR-0009): the rules of one file. The name as stored - no folders, no control characters, at most 200
  characters (the extension kept) - and the content type from the extension. Refuses a file without a name (U1120),
  of a type not on the list (U1121), empty (U1122) or over 20 MB (U1123).
*/
CREATE OR REPLACE FUNCTION crm.desk_attachment_normalize(p_file_name text, p_size_bytes bigint,
                                                         OUT p_name text, OUT p_content_type text)
    LANGUAGE plpgsql
    STABLE
AS
$$
DECLARE
    v_max_bytes bigint := 20 * 1024 * 1024;
    v_name      text;
    v_extension text;
BEGIN
    v_name := regexp_replace(replace(coalesce(p_file_name, ''), '\', '/'), '^.*/', '');
    v_name := btrim(regexp_replace(v_name, '[[:cntrl:]]', '', 'g'));
    IF v_name = '' THEN
        RAISE EXCEPTION 'The file has no name.' USING ERRCODE = 'U1120';
    END IF;

    v_extension := lower(substring(v_name FROM '\.[^.]*$'));
    IF length(v_name) > 200 THEN
        v_name := left(v_name, 200 - coalesce(length(v_extension), 0)) || coalesce(v_extension, '');
    END IF;
    p_name := v_name;

    SELECT t.content_type INTO p_content_type FROM crm.desk_attachment_types AS t WHERE t.extension = v_extension;
    IF p_content_type IS NULL THEN
        RAISE EXCEPTION 'The file "%" has a type that is not allowed. Allowed: images (PNG, JPG, GIF, WEBP), PDF, Office documents, TXT/CSV, ZIP, MP4/MOV.',
            v_name USING ERRCODE = 'U1121';
    END IF;
    IF coalesce(p_size_bytes, 0) <= 0 THEN
        RAISE EXCEPTION 'The file "%" is empty.', v_name USING ERRCODE = 'U1122';
    END IF;
    IF p_size_bytes > v_max_bytes THEN
        RAISE EXCEPTION 'The file "%" is too large (% MB). The limit is 20 MB.', v_name,
            rtrim(rtrim(to_char(p_size_bytes / 1048576.0, 'FM999990.0'), '0'), '.') USING ERRCODE = 'U1123';
    END IF;
END;
$$;

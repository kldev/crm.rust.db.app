/*
  Internal (ADR-0010): an account number as stored - upper case without spaces, a Polish 26-digit number gets "PL" -
  when its check digits are valid (ISO 13616, mod 97); NULL when it is not an IBAN.
*/
CREATE OR REPLACE FUNCTION crm.desk_iban_normalize(p_value text)
    RETURNS text
    LANGUAGE plpgsql
    IMMUTABLE
    PARALLEL SAFE
AS
$$
DECLARE
    v_iban       text := upper(replace(replace(replace(p_value, ' ', ''), chr(160), ''), chr(9), ''));
    v_rearranged text;
    v_remainder  int  := 0;
    v_c          text;
    v_digits     int;
BEGIN
    IF v_iban IS NULL THEN
        RETURN NULL;
    END IF;
    IF v_iban ~ '^[0-9]{26}$' THEN
        v_iban := 'PL' || v_iban;
    END IF;
    IF length(v_iban) NOT BETWEEN 15 AND 34 OR v_iban !~ '^[A-Z]{2}[A-Z0-9]*$' THEN
        RETURN NULL;
    END IF;
    v_rearranged := substr(v_iban, 5) || left(v_iban, 4);
    FOR i IN 1..length(v_rearranged)
        LOOP
            v_c := substr(v_rearranged, i, 1);
            v_digits := CASE WHEN v_c ~ '[0-9]' THEN ascii(v_c) - 48 ELSE ascii(v_c) - 55 END;
            v_remainder := CASE
                               WHEN v_digits >= 10 THEN (v_remainder * 100 + v_digits) % 97
                               ELSE (v_remainder * 10 + v_digits) % 97
                           END;
        END LOOP;
    RETURN CASE WHEN v_remainder = 1 THEN v_iban END;
END;
$$;

/*
  Internal (ADR-0010): the report data of a subcategory. p_values is a JSON object {field_key: value}, the whole set:
  a missing or empty value means "not given". Returns the values as stored (a JSON object of the given ones only)
  or refuses:
    U1130 a field of another subcategory      U1131 a required field is empty     U1132 over 200 characters
    U1133 not a date (yyyy-mm-dd)             U1134 not an e-mail                 U1135 not an amount (2 decimals)
    U1136 not an IBAN                         U1137 not a count 1-10000          U1138 a multi-choice value off the list
    U1139 a choice off the list               U1140 not a ticket number          U1141-U1145 identity document rules
  Stored forms: dates yyyy-mm-dd, amounts 1234.56, IBAN upper case without spaces, counts without separators,
  multi-choice "A, B" in the list's order, choices and ticket numbers as the list / upper case has them. Choices
  match regardless of letter case (as the SQL Server collation did).
*/
CREATE OR REPLACE FUNCTION crm.desk_fields_check(p_subcategory_key text, p_values jsonb)
    RETURNS jsonb
    LANGUAGE plpgsql
    STABLE
AS
$$
DECLARE
    v_in       jsonb := coalesce(p_values, '{}'::jsonb);
    v_out      jsonb := '{}'::jsonb;
    v_bad      text;
    d          record;
    v          text;
    v_result   text;
    v_date     date;
    v_compact  text;
    v_amount   numeric;
    v_picked   text[];
    v_document text;
    v_number   text;
BEGIN
    IF jsonb_typeof(v_in) <> 'object' THEN
        RAISE EXCEPTION 'Report fields must be a JSON object.' USING ERRCODE = 'U0004';
    END IF;

    SELECT k.key INTO v_bad
    FROM jsonb_object_keys(v_in) AS k (key)
    WHERE NOT EXISTS (SELECT 1
                      FROM crm.desk_subcategory_fields AS sf
                      WHERE sf.subcategory_key = p_subcategory_key
                        AND sf.field_key = k.key)
    LIMIT 1;
    IF v_bad IS NOT NULL THEN
        RAISE EXCEPTION 'The field "%" does not belong to this subcategory.', v_bad USING ERRCODE = 'U1130';
    END IF;

    FOR d IN
        SELECT f.field_key, f.label, f.kind, sf.is_required
        FROM crm.desk_subcategory_fields AS sf
        JOIN crm.desk_fields AS f ON f.field_key = sf.field_key
        WHERE sf.subcategory_key = p_subcategory_key
        ORDER BY sf.sort_order
        LOOP
            v := nullif(btrim(v_in ->> d.field_key), '');
            v_result := NULL;

            IF v IS NOT NULL AND length(v) > 200 THEN
                RAISE EXCEPTION 'The field "%" can have at most 200 characters.', d.label USING ERRCODE = 'U1132';
            END IF;

            IF v IS NOT NULL AND d.kind = 'MULTI_CHOICE' THEN
                SELECT array_agg(btrim(p)) INTO v_picked
                FROM unnest(string_to_array(v, ',')) AS p
                WHERE btrim(p) <> '';
                IF v_picked IS NULL THEN
                    v := NULL;
                END IF;
            END IF;

            IF v IS NULL THEN
                IF d.is_required THEN
                    RAISE EXCEPTION 'Fill in the field "%".', d.label USING ERRCODE = 'U1131';
                END IF;
                CONTINUE;
            END IF;

            CASE d.kind
                WHEN 'DATE' THEN v_date := NULL;
                                 IF v ~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}$' THEN
                                     BEGIN
                                         v_date := v::date;
                                     EXCEPTION
                                         WHEN others THEN v_date := NULL;
                                     END;
                                 END IF;
                                 IF v_date IS NULL THEN
                                     RAISE EXCEPTION 'The field "%" must be a date (yyyy-mm-dd).', d.label
                                         USING ERRCODE = 'U1133';
                                 END IF;
                                 v_result := to_char(v_date, 'YYYY-MM-DD');
                WHEN 'EMAIL' THEN IF v LIKE '% %' OR v NOT LIKE '_%@_%._%'
                    OR length(v) - length(replace(v, '@', '')) <> 1
                    OR v LIKE '%..%' OR v LIKE '%.' OR v LIKE '.%' OR v LIKE '%@.%' THEN
                                      RAISE EXCEPTION 'The field "%" must be an e-mail address.', d.label
                                          USING ERRCODE = 'U1134';
                                  END IF;
                                  v_result := v;
                WHEN 'AMOUNT' THEN
                    -- Spaces (also non-breaking) as thousands separators, a comma or a dot as the decimal point.
                    v_compact := replace(replace(replace(v, ' ', ''), chr(160), ''), ',', '.');
                    v_amount := NULL;
                    IF v_compact ~ '^[+-]?([0-9]+\.?[0-9]*|\.[0-9]+)$' THEN
                        v_amount := v_compact::numeric;
                    END IF;
                    IF v_amount IS NULL OR round(v_amount, 2) <> v_amount OR abs(v_amount) >= 1000000000000 THEN
                        RAISE EXCEPTION 'The field "%" must be an amount, e.g. 1 234.56 (at most 2 decimals).', d.label
                            USING ERRCODE = 'U1135';
                    END IF;
                    v_result := v_amount::numeric(15, 2)::text;
                WHEN 'IBAN' THEN v_result := crm.desk_iban_normalize(v);
                                 IF v_result IS NULL THEN
                                     RAISE EXCEPTION 'The field "%" is not a valid account number (IBAN).', d.label
                                         USING ERRCODE = 'U1136';
                                 END IF;
                WHEN 'COUNT' THEN v_compact := replace(v, ' ', '');
                                  -- Two steps: OR does not promise to test the pattern before the cast.
                                  IF v_compact !~ '^[0-9]{1,6}$' THEN
                                      v_compact := '0';
                                  END IF;
                                  IF v_compact::int NOT BETWEEN 1 AND 10000 THEN
                                      RAISE EXCEPTION 'The field "%" must be a number from 1 to 10000.', d.label
                                          USING ERRCODE = 'U1137';
                                  END IF;
                                  v_result := v_compact::int::text;
                WHEN 'MULTI_CHOICE' THEN SELECT p INTO v_bad
                                         FROM unnest(v_picked) AS p
                                         WHERE NOT EXISTS (SELECT 1
                                                           FROM crm.desk_field_options AS o
                                                           WHERE o.field_key = d.field_key
                                                             AND lower(o.option_value) = lower(p))
                                         LIMIT 1;
                                         IF FOUND THEN
                                             RAISE EXCEPTION '"%" is not on the list of the field "%".', v_bad, d.label
                                                 USING ERRCODE = 'U1138';
                                         END IF;
                                         SELECT string_agg(o.option_value, ', ' ORDER BY o.sort_order) INTO v_result
                                         FROM crm.desk_field_options AS o
                                         WHERE o.field_key = d.field_key
                                           AND lower(o.option_value) IN (SELECT lower(p) FROM unnest(v_picked) AS p);
                WHEN 'CHOICE' THEN SELECT o.option_value INTO v_result
                                   FROM crm.desk_field_options AS o
                                   WHERE o.field_key = d.field_key
                                     AND lower(o.option_value) = lower(v);
                                   IF v_result IS NULL THEN
                                       RAISE EXCEPTION 'Choose the value of the field "%" from the list.', d.label
                                           USING ERRCODE = 'U1139';
                                   END IF;
                WHEN 'TICKET_NUMBER' THEN v_result := upper(v);
                                          IF v_result !~ '^HAL-[0-9A-F]{8}$' THEN
                                              RAISE EXCEPTION 'The field "%" must be a ticket number, e.g. HAL-1A2B3C4D.',
                                                  d.label USING ERRCODE = 'U1140';
                                          END IF;
                ELSE v_result := v;
                END CASE;

            v_out := v_out || jsonb_build_object(d.field_key, v_result);
        END LOOP;

    -- The identity document and its number go together: a PESEL must be valid, "no documents" has no number.
    v_document := v_out ->> 'identity-document';
    v_number := v_out ->> 'identity-number';
    IF v_document IS NULL AND v_number IS NOT NULL THEN
        RAISE EXCEPTION 'Choose the type of document the number belongs to.' USING ERRCODE = 'U1141';
    END IF;
    IF v_document = 'Brak dokumentów' AND v_number IS NOT NULL THEN
        RAISE EXCEPTION 'With "no documents" no document number is given.' USING ERRCODE = 'U1142';
    END IF;
    IF v_document = 'PESEL' AND v_number IS NULL THEN
        RAISE EXCEPTION 'Enter the PESEL number.' USING ERRCODE = 'U1143';
    END IF;
    IF v_document = 'PESEL' AND NOT crm.desk_pesel_valid(v_number) THEN
        RAISE EXCEPTION 'The PESEL number is not valid (11 digits: a real date of birth and a correct check digit).' USING ERRCODE = 'U1144';
    END IF;
    IF v_document = 'Dowód osobisty' AND v_number IS NULL THEN
        RAISE EXCEPTION 'Enter the ID card number.' USING ERRCODE = 'U1145';
    END IF;
    IF v_document = 'Dowód osobisty' THEN
        v_out := v_out || jsonb_build_object('identity-number', upper(replace(v_number, ' ', '')));
    END IF;

    RETURN v_out;
END;
$$;

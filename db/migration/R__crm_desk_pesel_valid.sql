-- Internal (ADR-0010): a PESEL is 11 digits, the last one the check digit of weights 1-3-7-9, and the first six a real
-- date of birth (YYMMDD, the century in the month: +80 for 1800s, +0 for 1900s, +20, +40, +60 for 2000-2200s).
CREATE OR REPLACE FUNCTION crm.desk_pesel_valid(p_value text)
    RETURNS boolean
    LANGUAGE plpgsql
    IMMUTABLE
    PARALLEL SAFE
AS
$$
DECLARE
    v_weights int[] := ARRAY [1, 3, 7, 9, 1, 3, 7, 9, 1, 3];
    v_sum     int   := 0;
    v_month   int;
    v_day     int;
    v_year    int;
BEGIN
    IF p_value IS NULL OR p_value !~ '^[0-9]{11}$' THEN
        RETURN false;
    END IF;
    FOR i IN 1..10
        LOOP
            v_sum := v_sum + substr(p_value, i, 1)::int * v_weights[i];
        END LOOP;
    IF (10 - v_sum % 10) % 10 <> substr(p_value, 11, 1)::int THEN
        RETURN false;
    END IF;

    v_year := substr(p_value, 1, 2)::int;
    v_month := substr(p_value, 3, 2)::int;
    v_day := substr(p_value, 5, 2)::int;
    v_year := v_year + CASE v_month / 20
                           WHEN 4 THEN 1800
                           WHEN 0 THEN 1900
                           WHEN 1 THEN 2000
                           WHEN 2 THEN 2100
                           ELSE 2200 END;
    v_month := v_month % 20;
    IF v_month NOT BETWEEN 1 AND 12 OR v_day < 1 THEN
        RETURN false;
    END IF;
    RETURN v_day <= extract(DAY FROM make_date(v_year, v_month, 1) + interval '1 month - 1 day')::int;
END;
$$;

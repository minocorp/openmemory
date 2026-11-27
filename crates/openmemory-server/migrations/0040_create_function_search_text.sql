SET LOCAL lock_timeout = '5s';

CREATE FUNCTION search_text(input text) RETURNS text
LANGUAGE sql
IMMUTABLE
STRICT
PARALLEL SAFE
AS $$
  SELECT lower(normalize(input, NFC) COLLATE "und-x-icu")
$$;

CREATE TABLE page_fields (
  page_id bigint NOT NULL,
  key text NOT NULL,
  ord integer NOT NULL,
  text text NOT NULL,
  num numeric,
  date date,
  CONSTRAINT page_fields_pkey PRIMARY KEY (page_id, key, ord),
  CONSTRAINT page_fields_page_id_fkey FOREIGN KEY (page_id) REFERENCES pages (id) ON DELETE CASCADE,
  CONSTRAINT page_fields_key_check CHECK (length(key) BETWEEN 1 AND 64),
  CONSTRAINT page_fields_ord_check CHECK (ord >= 0),
  CONSTRAINT page_fields_text_check CHECK (octet_length(text) <= 2000),
  CONSTRAINT page_fields_typed_value_check CHECK (num IS NULL OR date IS NULL)
);

CREATE INDEX page_fields_key_text_idx ON page_fields (key, text);
CREATE INDEX page_fields_key_num_idx ON page_fields (key, num) WHERE num IS NOT NULL;
CREATE INDEX page_fields_key_date_idx ON page_fields (key, date) WHERE date IS NOT NULL;

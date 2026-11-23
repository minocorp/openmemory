CREATE TABLE file_pages (
  file_id bigint NOT NULL,
  page integer NOT NULL,
  text text NOT NULL,
  CONSTRAINT file_pages_pkey PRIMARY KEY (file_id, page),
  CONSTRAINT file_pages_file_id_fkey FOREIGN KEY (file_id) REFERENCES files (id) ON DELETE CASCADE,
  CONSTRAINT file_pages_page_check CHECK (page >= 1)
);

CREATE INDEX file_pages_text_bigm_idx ON file_pages USING gin (text gin_bigm_ops);

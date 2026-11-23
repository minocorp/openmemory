CREATE TABLE page_links (
  id bigint GENERATED ALWAYS AS IDENTITY,
  from_page_id bigint NOT NULL,
  to_page_id bigint,
  to_file_id bigint,
  CONSTRAINT page_links_pkey PRIMARY KEY (id),
  CONSTRAINT page_links_from_page_id_fkey FOREIGN KEY (from_page_id) REFERENCES pages (id) ON DELETE CASCADE,
  CONSTRAINT page_links_to_page_id_fkey FOREIGN KEY (to_page_id) REFERENCES pages (id) ON DELETE CASCADE,
  CONSTRAINT page_links_to_file_id_fkey FOREIGN KEY (to_file_id) REFERENCES files (id) ON DELETE CASCADE,
  CONSTRAINT page_links_from_page_id_to_page_id_key UNIQUE (from_page_id, to_page_id),
  CONSTRAINT page_links_from_page_id_to_file_id_key UNIQUE (from_page_id, to_file_id),
  CONSTRAINT page_links_target_check CHECK (num_nonnulls(to_page_id, to_file_id) = 1)
);

CREATE INDEX page_links_to_page_id_idx ON page_links (to_page_id) WHERE to_page_id IS NOT NULL;
CREATE INDEX page_links_to_file_id_idx ON page_links (to_file_id) WHERE to_file_id IS NOT NULL;

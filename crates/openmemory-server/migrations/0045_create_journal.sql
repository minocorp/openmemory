SET LOCAL lock_timeout = '5s';

CREATE TABLE journal (
  seq bigint GENERATED ALWAYS AS IDENTITY,
  page_id bigint,
  -- squawk-ignore prefer-bigint-over-int
  n integer,
  file_id bigint,
  file_op text,
  purge_id bigint,
  CONSTRAINT journal_pkey PRIMARY KEY (seq),
  CONSTRAINT journal_page_id_n_fkey FOREIGN KEY (page_id, n) REFERENCES page_versions (page_id, n) ON DELETE CASCADE,
  CONSTRAINT journal_file_id_fkey FOREIGN KEY (file_id) REFERENCES files (id) ON DELETE CASCADE,
  CONSTRAINT journal_purge_id_fkey FOREIGN KEY (purge_id) REFERENCES purges (id) ON DELETE RESTRICT,
  CONSTRAINT journal_page_id_n_key UNIQUE (page_id, n),
  CONSTRAINT journal_purge_id_key UNIQUE (purge_id),
  CONSTRAINT journal_subject_check CHECK (num_nonnulls(page_id, file_id, purge_id) = 1),
  CONSTRAINT journal_n_check CHECK ((page_id IS NULL) = (n IS NULL)),
  CONSTRAINT journal_file_check CHECK ((file_id IS NULL) = (file_op IS NULL)),
  CONSTRAINT journal_file_op_check CHECK (file_op IN ('upload', 'delete'))
);

-- squawk-ignore require-concurrent-index-creation
CREATE INDEX journal_file_id_idx ON journal (file_id);

CREATE TABLE page_versions (
  page_id bigint NOT NULL,
  n integer NOT NULL,
  seq bigint GENERATED ALWAYS AS IDENTITY,
  content text COMPRESSION lz4 NOT NULL,
  path text COLLATE "C" NOT NULL,
  op text NOT NULL,
  reverts_version integer,
  user_id bigint NOT NULL,
  connection_id bigint,
  author_name text NOT NULL,
  group_id bigint NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT page_versions_pkey PRIMARY KEY (page_id, n),
  CONSTRAINT page_versions_page_id_fkey FOREIGN KEY (page_id) REFERENCES pages (id) ON DELETE CASCADE,
  CONSTRAINT page_versions_reverts_fkey FOREIGN KEY (page_id, reverts_version) REFERENCES page_versions (page_id, n) ON DELETE CASCADE,
  CONSTRAINT page_versions_user_id_fkey FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE RESTRICT,
  CONSTRAINT page_versions_connection_id_fkey FOREIGN KEY (connection_id) REFERENCES connections (id) ON DELETE RESTRICT,
  CONSTRAINT page_versions_group_id_fkey FOREIGN KEY (group_id) REFERENCES change_groups (id) ON DELETE RESTRICT,
  CONSTRAINT page_versions_seq_key UNIQUE (seq),
  CONSTRAINT page_versions_n_check CHECK (n >= 1),
  CONSTRAINT page_versions_path_check CHECK (
    path ~ '^[^/]+(/[^/]+)*$'
    AND path !~ '(^|/)\.\.(/|$)'
    AND path !~ '[[:cntrl:]]'
    AND path IS NFC NORMALIZED
  ),
  CONSTRAINT page_versions_content_check CHECK (octet_length(content) <= 1048576),
  CONSTRAINT page_versions_op_check CHECK (op IN ('create', 'edit', 'move', 'delete', 'revert')),
  CONSTRAINT page_versions_reverts_version_check CHECK ((op = 'revert') = (reverts_version IS NOT NULL)),
  CONSTRAINT page_versions_author_name_check CHECK (length(author_name) > 0)
);

CREATE INDEX page_versions_group_id_idx ON page_versions (group_id);
CREATE INDEX page_versions_user_id_seq_idx ON page_versions (user_id, seq DESC);
CREATE INDEX page_versions_connection_id_idx ON page_versions (connection_id) WHERE connection_id IS NOT NULL;

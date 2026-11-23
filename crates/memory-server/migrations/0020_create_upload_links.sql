CREATE TABLE upload_links (
  id bigint GENERATED ALWAYS AS IDENTITY,
  token_hash bytea NOT NULL,
  path text COLLATE "C" NOT NULL,
  connection_id bigint NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  expires_at timestamptz NOT NULL,
  used_at timestamptz,
  CONSTRAINT upload_links_pkey PRIMARY KEY (id),
  CONSTRAINT upload_links_connection_id_fkey FOREIGN KEY (connection_id) REFERENCES connections (id) ON DELETE CASCADE,
  CONSTRAINT upload_links_token_hash_key UNIQUE (token_hash),
  CONSTRAINT upload_links_token_hash_check CHECK (length(token_hash) = 32),
  CONSTRAINT upload_links_path_check CHECK (
    path ~ '^[^/]+(/[^/]+)*$'
    AND path !~ '(^|/)\.\.(/|$)'
    AND path !~ '[[:cntrl:]]'
    AND path IS NFC NORMALIZED
  ),
  CONSTRAINT upload_links_expires_at_check CHECK (expires_at > created_at)
);

CREATE INDEX upload_links_connection_id_idx ON upload_links (connection_id);

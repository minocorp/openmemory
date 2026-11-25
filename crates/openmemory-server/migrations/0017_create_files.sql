CREATE TABLE files (
  id bigint GENERATED ALWAYS AS IDENTITY,
  path text COLLATE "C" NOT NULL,
  sha256 bytea NOT NULL,
  blob_key text NOT NULL,
  mime text NOT NULL,
  size bigint NOT NULL,
  user_id bigint NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  deleted_at timestamptz,
  CONSTRAINT files_pkey PRIMARY KEY (id),
  CONSTRAINT files_user_id_fkey FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE RESTRICT,
  CONSTRAINT files_path_check CHECK (
    path ~ '^[^/]+(/[^/]+)*$'
    AND path !~ '(^|/)\.\.(/|$)'
    AND path !~ '[[:cntrl:]]'
    AND path IS NFC NORMALIZED
  ),
  CONSTRAINT files_sha256_check CHECK (length(sha256) = 32),
  CONSTRAINT files_blob_key_check CHECK (length(blob_key) > 0),
  CONSTRAINT files_mime_check CHECK (mime ~ '^[^/[:space:]]+/[^/[:space:]]+$'),
  CONSTRAINT files_size_check CHECK (size >= 0)
);

CREATE UNIQUE INDEX files_path_live_key ON files (path) WHERE deleted_at IS NULL;
CREATE INDEX files_blob_key_idx ON files (blob_key);
CREATE INDEX files_user_id_idx ON files (user_id);

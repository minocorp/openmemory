CREATE TABLE pages (
  id bigint GENERATED ALWAYS AS IDENTITY,
  path text COLLATE "C" NOT NULL,
  current_version integer NOT NULL,
  content text COMPRESSION lz4 NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  deleted_at timestamptz,
  CONSTRAINT pages_pkey PRIMARY KEY (id),
  CONSTRAINT pages_path_check CHECK (
    path ~ '^[^/]+(/[^/]+)*$'
    AND path !~ '(^|/)\.\.(/|$)'
    AND path !~ '[[:cntrl:]]'
    AND path IS NFC NORMALIZED
  ),
  CONSTRAINT pages_current_version_check CHECK (current_version >= 1),
  CONSTRAINT pages_content_check CHECK (octet_length(content) <= 1048576)
);

CREATE UNIQUE INDEX pages_path_live_key ON pages (path) WHERE deleted_at IS NULL;
CREATE INDEX pages_content_bigm_idx ON pages USING gin (content gin_bigm_ops) WHERE deleted_at IS NULL;

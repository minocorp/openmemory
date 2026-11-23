CREATE TABLE oauth_codes (
  id bigint GENERATED ALWAYS AS IDENTITY,
  code_hash bytea NOT NULL,
  oauth_client_id bigint NOT NULL,
  connection_id bigint NOT NULL,
  redirect_uri text NOT NULL,
  code_challenge text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  expires_at timestamptz NOT NULL,
  used_at timestamptz,
  CONSTRAINT oauth_codes_pkey PRIMARY KEY (id),
  CONSTRAINT oauth_codes_oauth_client_id_fkey FOREIGN KEY (oauth_client_id) REFERENCES oauth_clients (id) ON DELETE CASCADE,
  CONSTRAINT oauth_codes_connection_id_fkey FOREIGN KEY (connection_id) REFERENCES connections (id) ON DELETE CASCADE,
  CONSTRAINT oauth_codes_code_hash_key UNIQUE (code_hash),
  CONSTRAINT oauth_codes_code_hash_check CHECK (length(code_hash) = 32),
  CONSTRAINT oauth_codes_expires_at_check CHECK (expires_at > created_at)
);

CREATE INDEX oauth_codes_oauth_client_id_idx ON oauth_codes (oauth_client_id);
CREATE INDEX oauth_codes_connection_id_idx ON oauth_codes (connection_id);

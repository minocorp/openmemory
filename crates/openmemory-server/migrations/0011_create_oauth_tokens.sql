CREATE TABLE oauth_tokens (
  id bigint GENERATED ALWAYS AS IDENTITY,
  connection_id bigint NOT NULL,
  oauth_client_id bigint NOT NULL,
  access_token_hash bytea NOT NULL,
  refresh_token_hash bytea NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  access_expires_at timestamptz NOT NULL,
  refresh_expires_at timestamptz NOT NULL,
  rotated_at timestamptz,
  CONSTRAINT oauth_tokens_pkey PRIMARY KEY (id),
  CONSTRAINT oauth_tokens_connection_id_fkey FOREIGN KEY (connection_id) REFERENCES connections (id) ON DELETE CASCADE,
  CONSTRAINT oauth_tokens_oauth_client_id_fkey FOREIGN KEY (oauth_client_id) REFERENCES oauth_clients (id) ON DELETE CASCADE,
  CONSTRAINT oauth_tokens_access_token_hash_key UNIQUE (access_token_hash),
  CONSTRAINT oauth_tokens_refresh_token_hash_key UNIQUE (refresh_token_hash),
  CONSTRAINT oauth_tokens_access_token_hash_check CHECK (length(access_token_hash) = 32),
  CONSTRAINT oauth_tokens_refresh_token_hash_check CHECK (length(refresh_token_hash) = 32),
  CONSTRAINT oauth_tokens_expires_at_check CHECK (access_expires_at > created_at AND refresh_expires_at >= access_expires_at)
);

CREATE INDEX oauth_tokens_connection_id_idx ON oauth_tokens (connection_id);
CREATE INDEX oauth_tokens_oauth_client_id_idx ON oauth_tokens (oauth_client_id);

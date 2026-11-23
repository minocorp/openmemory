CREATE TABLE oauth_consents (
  id bigint GENERATED ALWAYS AS IDENTITY,
  token_hash bytea NOT NULL,
  oauth_client_id bigint NOT NULL,
  redirect_uri text NOT NULL,
  code_challenge text NOT NULL,
  client_state text,
  created_at timestamptz NOT NULL DEFAULT now(),
  expires_at timestamptz NOT NULL,
  CONSTRAINT oauth_consents_pkey PRIMARY KEY (id),
  CONSTRAINT oauth_consents_oauth_client_id_fkey FOREIGN KEY (oauth_client_id) REFERENCES oauth_clients (id) ON DELETE CASCADE,
  CONSTRAINT oauth_consents_token_hash_key UNIQUE (token_hash),
  CONSTRAINT oauth_consents_token_hash_check CHECK (length(token_hash) = 32),
  CONSTRAINT oauth_consents_code_challenge_check CHECK (length(code_challenge) = 43),
  CONSTRAINT oauth_consents_expires_at_check CHECK (expires_at > created_at)
);

CREATE INDEX oauth_consents_oauth_client_id_idx ON oauth_consents (oauth_client_id);

CREATE TABLE passkeys (
  id bigint GENERATED ALWAYS AS IDENTITY,
  user_id bigint NOT NULL,
  credential_id bytea NOT NULL,
  credential jsonb NOT NULL,
  name text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  last_used_at timestamptz,
  CONSTRAINT passkeys_pkey PRIMARY KEY (id),
  CONSTRAINT passkeys_user_id_fkey FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
  CONSTRAINT passkeys_credential_id_key UNIQUE (credential_id),
  CONSTRAINT passkeys_credential_check CHECK (jsonb_typeof(credential) = 'object'),
  CONSTRAINT passkeys_name_check CHECK (length(name) BETWEEN 1 AND 64)
);

CREATE INDEX passkeys_user_id_idx ON passkeys (user_id);

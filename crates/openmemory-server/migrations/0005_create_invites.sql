CREATE TABLE invites (
  id bigint GENERATED ALWAYS AS IDENTITY,
  inviter_id bigint NOT NULL,
  token_hash bytea NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  expires_at timestamptz NOT NULL,
  used_at timestamptz,
  CONSTRAINT invites_pkey PRIMARY KEY (id),
  CONSTRAINT invites_inviter_id_fkey FOREIGN KEY (inviter_id) REFERENCES users (id) ON DELETE CASCADE,
  CONSTRAINT invites_token_hash_key UNIQUE (token_hash),
  CONSTRAINT invites_token_hash_check CHECK (length(token_hash) = 32),
  CONSTRAINT invites_expires_at_check CHECK (expires_at > created_at)
);

CREATE INDEX invites_inviter_id_idx ON invites (inviter_id);

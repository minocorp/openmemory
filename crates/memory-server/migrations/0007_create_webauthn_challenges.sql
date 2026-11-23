CREATE TABLE webauthn_challenges (
  id bigint GENERATED ALWAYS AS IDENTITY,
  token_hash bytea NOT NULL,
  kind text NOT NULL,
  user_id bigint,
  state jsonb NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  expires_at timestamptz NOT NULL,
  CONSTRAINT webauthn_challenges_pkey PRIMARY KEY (id),
  CONSTRAINT webauthn_challenges_user_id_fkey FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
  CONSTRAINT webauthn_challenges_token_hash_key UNIQUE (token_hash),
  CONSTRAINT webauthn_challenges_token_hash_check CHECK (length(token_hash) = 32),
  CONSTRAINT webauthn_challenges_kind_check CHECK (kind IN ('registration', 'authentication')),
  CONSTRAINT webauthn_challenges_state_check CHECK (jsonb_typeof(state) = 'object'),
  CONSTRAINT webauthn_challenges_expires_at_check CHECK (expires_at > created_at)
);

CREATE INDEX webauthn_challenges_user_id_idx ON webauthn_challenges (user_id);

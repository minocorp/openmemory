CREATE TABLE recovery_codes (
  id bigint GENERATED ALWAYS AS IDENTITY,
  user_id bigint NOT NULL,
  code_hash bytea NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  used_at timestamptz,
  CONSTRAINT recovery_codes_pkey PRIMARY KEY (id),
  CONSTRAINT recovery_codes_user_id_fkey FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
  CONSTRAINT recovery_codes_code_hash_key UNIQUE (code_hash),
  CONSTRAINT recovery_codes_code_hash_check CHECK (length(code_hash) = 32)
);

CREATE UNIQUE INDEX recovery_codes_user_id_unused_key ON recovery_codes (user_id) WHERE used_at IS NULL;

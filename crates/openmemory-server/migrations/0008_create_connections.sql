CREATE TABLE connections (
  id bigint GENERATED ALWAYS AS IDENTITY,
  user_id bigint NOT NULL,
  name text NOT NULL,
  scopes text[] NOT NULL,
  can_write boolean NOT NULL DEFAULT true,
  created_at timestamptz NOT NULL DEFAULT now(),
  revoked_at timestamptz,
  CONSTRAINT connections_pkey PRIMARY KEY (id),
  CONSTRAINT connections_user_id_fkey FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
  CONSTRAINT connections_name_check CHECK (length(name) BETWEEN 1 AND 64 AND name !~ '[/[:cntrl:]]'),
  CONSTRAINT connections_scopes_check CHECK (cardinality(scopes) > 0 AND array_position(scopes, NULL) IS NULL)
);

CREATE UNIQUE INDEX connections_user_id_name_live_key ON connections (user_id, name) WHERE revoked_at IS NULL;

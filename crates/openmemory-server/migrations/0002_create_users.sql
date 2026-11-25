CREATE TABLE users (
  id bigint GENERATED ALWAYS AS IDENTITY,
  name text NOT NULL,
  is_admin boolean NOT NULL DEFAULT false,
  last_seen_seq bigint NOT NULL DEFAULT 0,
  created_at timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT users_pkey PRIMARY KEY (id),
  CONSTRAINT users_name_key UNIQUE (name),
  CONSTRAINT users_name_check CHECK (length(name) BETWEEN 1 AND 64 AND name !~ '[/[:cntrl:]]'),
  CONSTRAINT users_last_seen_seq_check CHECK (last_seen_seq >= 0)
);

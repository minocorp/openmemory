CREATE TABLE oauth_clients (
  id bigint GENERATED ALWAYS AS IDENTITY,
  client_id text NOT NULL,
  name text NOT NULL,
  redirect_uris text[] NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT oauth_clients_pkey PRIMARY KEY (id),
  CONSTRAINT oauth_clients_client_id_key UNIQUE (client_id),
  CONSTRAINT oauth_clients_name_check CHECK (length(name) BETWEEN 1 AND 128),
  CONSTRAINT oauth_clients_redirect_uris_check CHECK (cardinality(redirect_uris) > 0 AND array_position(redirect_uris, NULL) IS NULL)
);

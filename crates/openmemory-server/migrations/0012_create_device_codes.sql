CREATE TABLE device_codes (
  id bigint GENERATED ALWAYS AS IDENTITY,
  device_code_hash bytea NOT NULL,
  user_code text NOT NULL,
  oauth_client_id bigint NOT NULL,
  connection_id bigint,
  created_at timestamptz NOT NULL DEFAULT now(),
  expires_at timestamptz NOT NULL,
  CONSTRAINT device_codes_pkey PRIMARY KEY (id),
  CONSTRAINT device_codes_oauth_client_id_fkey FOREIGN KEY (oauth_client_id) REFERENCES oauth_clients (id) ON DELETE CASCADE,
  CONSTRAINT device_codes_connection_id_fkey FOREIGN KEY (connection_id) REFERENCES connections (id) ON DELETE CASCADE,
  CONSTRAINT device_codes_device_code_hash_key UNIQUE (device_code_hash),
  CONSTRAINT device_codes_user_code_key UNIQUE (user_code),
  CONSTRAINT device_codes_device_code_hash_check CHECK (length(device_code_hash) = 32),
  CONSTRAINT device_codes_expires_at_check CHECK (expires_at > created_at)
);

CREATE INDEX device_codes_oauth_client_id_idx ON device_codes (oauth_client_id);
CREATE INDEX device_codes_connection_id_idx ON device_codes (connection_id) WHERE connection_id IS NOT NULL;

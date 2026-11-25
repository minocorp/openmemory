SET LOCAL lock_timeout = '5s';

ALTER TABLE connections
  ADD COLUMN audience text NOT NULL DEFAULT 'mcp',
  ADD COLUMN expires_at timestamptz,
  ADD COLUMN personal_token_hash bytea,
  ADD CONSTRAINT connections_audience_check CHECK (audience IN ('mcp', 'api')) NOT VALID,
  ADD CONSTRAINT connections_personal_token_hash_check CHECK (personal_token_hash IS NULL OR length(personal_token_hash) = 32) NOT VALID;

ALTER TABLE oauth_consents
  ADD COLUMN audience text NOT NULL DEFAULT 'mcp',
  ADD CONSTRAINT oauth_consents_audience_check CHECK (audience IN ('mcp', 'api')) NOT VALID;

ALTER TABLE device_codes
  ADD COLUMN audience text NOT NULL DEFAULT 'mcp',
  ADD CONSTRAINT device_codes_audience_check CHECK (audience IN ('mcp', 'api')) NOT VALID;

INSERT INTO oauth_clients (client_id, name, redirect_uris)
VALUES ('memory-cli', 'Memory CLI', ARRAY['http://127.0.0.1/callback', 'http://[::1]/callback'])
ON CONFLICT (client_id) DO NOTHING;

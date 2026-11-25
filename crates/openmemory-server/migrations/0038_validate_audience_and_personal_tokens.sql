SET LOCAL lock_timeout = '5s';

ALTER TABLE connections
  ADD CONSTRAINT connections_personal_token_hash_key UNIQUE USING INDEX connections_personal_token_hash_key,
  VALIDATE CONSTRAINT connections_audience_check,
  VALIDATE CONSTRAINT connections_personal_token_hash_check;

ALTER TABLE oauth_consents VALIDATE CONSTRAINT oauth_consents_audience_check;
ALTER TABLE device_codes VALIDATE CONSTRAINT device_codes_audience_check;

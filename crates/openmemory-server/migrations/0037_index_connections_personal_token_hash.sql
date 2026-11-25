-- no-transaction
-- squawk-ignore require-lock-timeout
CREATE UNIQUE INDEX CONCURRENTLY connections_personal_token_hash_key ON connections (personal_token_hash);

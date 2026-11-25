-- no-transaction
-- squawk-ignore require-lock-timeout
CREATE UNIQUE INDEX CONCURRENTLY users_username_key ON users (username);

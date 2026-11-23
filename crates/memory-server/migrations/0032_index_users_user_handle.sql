-- no-transaction
-- squawk-ignore require-lock-timeout
CREATE UNIQUE INDEX CONCURRENTLY users_user_handle_key ON users (user_handle);

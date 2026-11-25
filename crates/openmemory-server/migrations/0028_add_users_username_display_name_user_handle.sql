SET LOCAL lock_timeout = '5s';

ALTER TABLE users
  ADD COLUMN username text,
  ADD COLUMN display_name text,
  ADD COLUMN user_handle bytea;

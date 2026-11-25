SET LOCAL lock_timeout = '5s';

ALTER TABLE users
  ADD CONSTRAINT users_username_key UNIQUE USING INDEX users_username_key,
  ADD CONSTRAINT users_user_handle_key UNIQUE USING INDEX users_user_handle_key,
  ADD CONSTRAINT users_username_check CHECK (username ~ '^[a-z0-9_.-]{1,32}$') NOT VALID,
  ADD CONSTRAINT users_display_name_check CHECK (length(display_name) BETWEEN 1 AND 64 AND display_name !~ '[[:cntrl:]]') NOT VALID,
  ADD CONSTRAINT users_user_handle_check CHECK (length(user_handle) = 16) NOT VALID;

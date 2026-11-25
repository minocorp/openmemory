SET LOCAL lock_timeout = '5s';

ALTER TABLE users
  VALIDATE CONSTRAINT users_username_check,
  VALIDATE CONSTRAINT users_display_name_check,
  VALIDATE CONSTRAINT users_user_handle_check;

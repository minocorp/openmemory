DO $$
DECLARE
  account record;
  base text;
  candidate text;
  attempt integer;
BEGIN
  FOR account IN SELECT id, name FROM users ORDER BY id LOOP
    base := trim(both '-' FROM regexp_replace(lower(account.name), '[^a-z0-9_.-]+', '-', 'g'));
    base := trim(both '-' FROM left(base, 32));
    IF base = '' THEN
      base := 'user';
    END IF;
    candidate := base;
    attempt := 1;
    WHILE EXISTS (SELECT 1 FROM users WHERE username = candidate) LOOP
      attempt := attempt + 1;
      candidate := left(base, 32 - length(attempt::text) - 1) || '-' || attempt;
    END LOOP;
    UPDATE users
    SET username = candidate,
        display_name = account.name,
        user_handle = uuid_send(gen_random_uuid())
    WHERE id = account.id;
  END LOOP;
END
$$;

-- squawk-ignore require-lock-timeout
CREATE FUNCTION journal_append() RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
  -- Runs at commit and holds the lock until the commit ends, so seq order is commit order.
  PERFORM pg_advisory_xact_lock(hashtext('journal'), 0);
  IF TG_TABLE_NAME = 'page_versions' THEN
    INSERT INTO journal (page_id, n) VALUES (NEW.page_id, NEW.n);
  ELSIF TG_TABLE_NAME = 'files' THEN
    INSERT INTO journal (file_id, file_op)
    VALUES (NEW.id, CASE TG_OP WHEN 'INSERT' THEN 'upload' ELSE 'delete' END);
  ELSE
    INSERT INTO journal (purge_id) VALUES (NEW.id);
  END IF;
  PERFORM pg_notify('journal', '');
  RETURN NULL;
END;
$$;

SET LOCAL lock_timeout = '5s';

-- squawk-ignore ban-drop-column
ALTER TABLE users DROP COLUMN name;

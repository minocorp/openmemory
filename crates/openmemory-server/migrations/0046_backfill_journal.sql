SET LOCAL lock_timeout = '5s';

INSERT INTO journal (seq, page_id, n)
OVERRIDING SYSTEM VALUE
SELECT
  seq,
  page_id,
  n
FROM page_versions
ORDER BY seq;

SELECT setval(pg_get_serial_sequence('journal', 'seq'), max(seq))
FROM journal
HAVING max(seq) IS NOT NULL;

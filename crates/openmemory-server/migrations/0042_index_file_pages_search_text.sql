-- no-transaction
-- squawk-ignore require-lock-timeout
CREATE INDEX CONCURRENTLY file_pages_search_text_idx ON file_pages USING gin (search_text(text) gin_bigm_ops);

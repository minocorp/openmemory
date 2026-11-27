-- no-transaction
-- squawk-ignore require-lock-timeout
CREATE INDEX CONCURRENTLY files_search_text_idx ON files USING gin (search_text(path) gin_bigm_ops) WHERE deleted_at IS NULL;

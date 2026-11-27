-- no-transaction
-- squawk-ignore require-lock-timeout
CREATE INDEX CONCURRENTLY pages_search_text_idx ON pages USING gin (search_text(path || E'\n' || content) gin_bigm_ops) WHERE deleted_at IS NULL;

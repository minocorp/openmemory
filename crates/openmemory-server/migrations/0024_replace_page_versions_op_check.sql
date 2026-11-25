ALTER TABLE page_versions DROP CONSTRAINT page_versions_op_check;
ALTER TABLE page_versions
  ADD CONSTRAINT page_versions_op_check CHECK (op IN ('create', 'edit', 'move', 'relink', 'delete', 'revert')) NOT VALID;

CREATE TABLE change_groups (
  id bigint GENERATED ALWAYS AS IDENTITY,
  user_id bigint NOT NULL,
  connection_id bigint,
  task text,
  created_at timestamptz NOT NULL DEFAULT now(),
  last_at timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT change_groups_pkey PRIMARY KEY (id),
  CONSTRAINT change_groups_user_id_fkey FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE RESTRICT,
  CONSTRAINT change_groups_connection_id_fkey FOREIGN KEY (connection_id) REFERENCES connections (id) ON DELETE RESTRICT,
  CONSTRAINT change_groups_task_check CHECK (length(task) BETWEEN 1 AND 200),
  CONSTRAINT change_groups_last_at_check CHECK (last_at >= created_at)
);

CREATE INDEX change_groups_user_id_last_at_idx ON change_groups (user_id, last_at DESC);
CREATE INDEX change_groups_connection_id_last_at_idx ON change_groups (connection_id, last_at DESC) WHERE connection_id IS NOT NULL;

CREATE TABLE purges (
  id bigint GENERATED ALWAYS AS IDENTITY,
  kind text NOT NULL,
  subject_id bigint NOT NULL,
  path text COLLATE "C" NOT NULL,
  user_id bigint NOT NULL,
  purged_at timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT purges_pkey PRIMARY KEY (id),
  CONSTRAINT purges_kind_subject_id_key UNIQUE (kind, subject_id),
  CONSTRAINT purges_user_id_fkey FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE RESTRICT,
  CONSTRAINT purges_kind_check CHECK (kind IN ('page', 'file'))
);

CREATE INDEX purges_user_id_idx ON purges (user_id);

# SQL style (Postgres 17, pg_bigm, sqlx)

## Identifiers
- Lowercase snake_case, always unquoted; never reserved words or mixed case ([identifiers](https://www.postgresql.org/docs/17/sql-syntax-lexical.html#SQL-SYNTAX-IDENTIFIERS)).
- Tables plural (`pages`, `page_versions`); join tables name both sides (`file_pages`).
- Columns singular. Foreign keys `<singular>_id`. Timestamps `<event>_at`. Booleans `is_*` or `has_*`.
- Keep an event name for nullable timestamps instead of a boolean: `deleted_at`, `revoked_at`.
- No table-name prefix on columns (`pages.path`, not `pages.page_path`); a primary key is plain `id`.
- Constraint and index names are explicit, never generated:

| Kind | Pattern | Example |
|---|---|---|
| primary key | `<table>_pkey` | `pages_pkey` |
| foreign key | `<table>_<col>_fkey` | `page_versions_page_id_fkey` |
| unique | `<table>_<cols>_key` | `users_email_key` |
| partial unique | `<table>_<cols>_<predicate>_key` | `pages_path_live_key` |
| check | `<table>_<rule>_check` | `files_size_check` |
| index | `<table>_<cols>_idx` | `page_versions_page_id_n_idx` |

## Types
Source: [Don't Do This](https://wiki.postgresql.org/wiki/Don't_Do_This).
- `timestamptz` for every instant; never `timestamp`. Default `now()`; UTC is a client concern.
- `text` for strings; never `varchar(n)` or `char(n)`. Limits go in `CHECK (length(x) <= n)` only when a real rule exists.
- `bigint GENERATED ALWAYS AS IDENTITY`; never `serial` or `bigserial`.
- `numeric` for exact decimals, never `money`; `double precision` only for measurements.
- `boolean`, never 0/1 or text flags. `bytea` for hashes (`sha256 bytea`), not hex text.
- `date` for calendar dates, never a timestamp truncated by convention.
- No `BETWEEN` on timestamps; use `>= a AND < b`.
- Never `NOT IN (subquery)`; use `NOT EXISTS`.

## Primary keys
- `bigint` identity, not uuid. Reasons: 8 bytes, append-ordered btree, and the id is not a secret because every request is authorized by user and connection.
- Postgres 17 has no native `uuidv7()` ([uuidv7 lands in 18](https://www.postgresql.org/docs/18/functions-uuid.html)); app-side generation adds a dependency for no gain here.
- Ids exposed in the web API are the same `id` column; never invent a second public id.
- Join tables use a composite primary key, no surrogate id.
- Insertion order that must be globally total (`seq`) is its own identity column, not `id` arithmetic.

## Constraints
- Every column `NOT NULL` unless absence is a real state; `NULL` never means "empty string" or "zero".
- Defaults live in the schema only when every writer wants them.
- Every foreign key has an explicit `ON DELETE` (`CASCADE`, `RESTRICT`, or `SET NULL`); never rely on the default.
- Every foreign key column is indexed unless it is the leading column of an existing index or primary key ([Postgres does not auto-index referencing columns](https://www.postgresql.org/docs/17/ddl-constraints.html#DDL-CONSTRAINTS-FK)).
- Invariants the app already checks are also `CHECK`s: `CHECK (size >= 0)`, `CHECK (op IN ('create', 'edit'))`.
- Soft delete: `deleted_at timestamptz`; uniqueness among live rows is a partial unique index:
```sql
CREATE UNIQUE INDEX pages_path_live_key ON pages (path) WHERE deleted_at IS NULL;
```
- Prefer `UNIQUE` constraints for whole-table uniqueness, partial unique indexes only for predicates.

## Enums
- `text` + named `CHECK`, not `CREATE TYPE ... AS ENUM`. Reason: values can be added or removed with one `ALTER TABLE`, while enum values cannot be removed and a new value cannot be used in the transaction that adds it ([ALTER TYPE](https://www.postgresql.org/docs/17/sql-altertype.html)); sqlx maps `text` to `String` with no custom type.
- Rust side: a Rust enum with `sqlx::Type` + `#[sqlx(type_name = "text")]` or a manual `TryFrom<String>`; the CHECK and the Rust enum list the same values.

## JSONB
- Only for opaque documents the database never filters, joins, or constrains.
- Anything queried by key is a column or a child table (`page_fields`), not a JSONB path.
- If a JSONB column is queried, it has a `CHECK (jsonb_typeof(x) = 'object')` and a documented GIN index; otherwise drop the idea.
- Never store secrets or ids that need foreign keys in JSONB.

## Indexes
- One index per proven query; justify each in the migration name or PR. Check with `EXPLAIN (ANALYZE, BUFFERS)` on realistic data.
- Column order: equality columns first, range or sort column last.
- Partial indexes for soft-deleted or rarely true predicates; the query must repeat the exact predicate.
- Prefix search (`LIKE 'a/b/%'`): the column is `COLLATE "C"` or the index uses `text_pattern_ops` ([operator classes](https://www.postgresql.org/docs/17/indexes-opclass.html)); otherwise a non-C locale cannot use the btree.
- Substring search (pg_bigm): `CREATE INDEX ... USING gin (content gin_bigm_ops)` ([pg_bigm](https://github.com/pgbigm/pg_bigm)); only on text that is searched, never on snapshots.
- `CREATE EXTENSION pg_bigm` lives in its own migration.
- Do not add an index that duplicates the leading columns of another.

## Migrations
- Location `crates/memory-server/migrations/`, forward-only, created with `sqlx migrate add --sequential <name>` ([sqlx-cli](https://docs.rs/sqlx-cli)), producing `0001_name.sql`.
- Name = verb + object, snake_case: `0007_add_page_links.sql`, `0008_index_files_sha256.sql`.
- One concern per file: one table, one index, or one data backfill.
- Never edit or renumber an applied migration; sqlx stores a checksum and refuses a mismatch ([Migrator](https://docs.rs/sqlx/latest/sqlx/migrate/struct.Migrator.html)). Fix with a new migration.
- No down migrations; recover by restore, not by reverse scripts.
- Each file runs in one transaction by default; so DDL is atomic, and a failed migration leaves nothing behind.
- `CREATE INDEX CONCURRENTLY` cannot run in a transaction: put it alone in a file whose first line is `-- no-transaction` (sqlx directive, [docs](https://docs.rs/sqlx/latest/sqlx/macro.migrate.html)). A failed concurrent build leaves an `INVALID` index; drop it before retrying.
- Plain `CREATE INDEX` is fine in the initial schema and on small tables; concurrent only for tables that hold data in production.
- Safe changes on live tables:
  - Add a column as nullable, or with a constant default (instant on 11+).
  - Add a constraint as `NOT VALID`, then `VALIDATE CONSTRAINT` in the next file.
  - Rename or drop in two releases: stop using, then drop.
  - Never change a column type in place on a large table; add, backfill, swap.
- Set `lock_timeout` at the top of any migration that takes `ACCESS EXCLUSIVE`: `SET LOCAL lock_timeout = '5s';`.
- Backfills in their own file, batched when over ~100k rows.
- Migrations contain no seed data except rows the app cannot start without.

## Formatting
- Keywords uppercase, identifiers and functions lowercase. Reason: SQL sits in Rust string literals with no syntax highlighting, so case is the only visual separation.
- One clause per line; clause keyword at line start; two-space indent for continuations.
- One select-list item, one column definition, one condition per line when more than one fits on a line.
- Commas at line end; no leading or trailing commas in lists.
- `AS` always spelled for aliases; aliases are the singular noun initial only when unambiguous, otherwise the full table name.
- Qualify every column with its alias once there is a join.
- Write `JOIN` and `LEFT JOIN`, never `INNER JOIN` or comma joins.
- CTEs named by what they hold (`live_pages`), used only when they remove repetition or isolate a data-modifying step.
```sql
SELECT p.id, p.path, v.created_at
FROM pages AS p
JOIN page_versions AS v ON v.page_id = p.id AND v.n = p.current_version
WHERE p.deleted_at IS NULL
  AND p.path LIKE $1
ORDER BY v.created_at DESC
LIMIT $2;
```

## Queries
- Explicit column lists; never `SELECT *` or `RETURNING *`. Adding a column must not change a result shape.
- Values are always bound parameters (`$1`); never `format!` into SQL. Identifiers from users are never accepted; map to a fixed set in Rust.
- Prefer `sqlx::query!` / `query_as!` (compile-time checked). `QueryBuilder` only where the shape is dynamic (the `where` grammar), pushing values with `push_bind`, fragments only from static strings.
- Commit `.sqlx/` (`cargo sqlx prepare --workspace -- --all-targets`, test queries included) and build CI with `SQLX_OFFLINE=true`; CI also checks it is current with `cargo sqlx prepare --check`.
- Nullability overrides in the select list name the intent: `col AS "col!"` (never null), `col AS "col?"` (nullable).
- `LIKE`: escape user text first (`\`, `%`, `_`), using `ESCAPE '\'`. For pg_bigm use `likequery()` or the same escaper; one shared Rust function does it.
- Search results: always `LIMIT`; pagination by keyset (`WHERE (created_at, id) < ($1, $2)`), not `OFFSET`.
- Strip NUL bytes from text before insert; `text` rejects them.
- No N+1: fetch children with `WHERE parent_id = ANY($1)` (bind a `Vec<i64>`) or a join; never query inside a loop.
- Bulk insert with `UNNEST($1::bigint[], $2::text[])`, not a loop of inserts.
- Upserts name the conflict target: `ON CONFLICT (page_id, key) DO UPDATE SET ...`.
- `EXISTS (SELECT 1 ...)` for presence; `COUNT(*)` only when the number is the answer.
- Order by something total (`ORDER BY created_at DESC, id DESC`) whenever order is observable.

## Transactions and locks
- A multi-statement change is one transaction (`pool.begin()`), committed explicitly; early return rolls back.
- Read-then-write on a row: `SELECT ... FOR UPDATE` in the same transaction; optimistic checks use a version column and `WHERE current_version = $n`, then check `rows_affected()`.
- Lock rows in a consistent order (by `id`) to avoid deadlocks.
- Keep transactions short: no network, filesystem, or user wait inside.
- Use the default `READ COMMITTED`; stronger isolation only with a retry loop on `40001`.
- Allocate global order with identity columns, never `MAX(x) + 1`.
- Queue claims: `FOR UPDATE SKIP LOCKED`.

## Tooling
- `squawk` ([squawkhq.com](https://squawkhq.com)) in CI on new migrations only: it flags blocking-lock changes and non-concurrent indexes. Reason: it encodes the migration rules above mechanically. `.squawk.toml` excludes `prefer-robust-stmts` and `require-statement-timeout`: migrations are transactional and forward-only, so they are never rerun.
- No SQL formatter or sqlfluff: queries live in Rust strings where formatters do not run, and migrations are few; review enforces the Formatting section.
- Schema truth is `migrations/`; no hand-maintained schema dump.

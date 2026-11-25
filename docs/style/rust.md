# Rust style

Baseline: [Rust Style Guide](https://doc.rust-lang.org/style-guide/) (enforced by rustfmt) and [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/). Where this file is silent, those win.

## Tooling

`rust-toolchain.toml` ([rustup overrides](https://rust-lang.github.io/rustup/overrides.html)):

```toml
[toolchain]
channel = "1.100.0"
components = ["rustfmt", "clippy"]
profile = "minimal"
```

`rustfmt.toml` (stable options only; `group_imports`/`imports_granularity` are nightly, do not use; [options](https://rust-lang.github.io/rustfmt/)):

```toml
edition = "2024"
newline_style = "Unix"
use_field_init_shorthand = true
```

Root `Cargo.toml` ([lints table](https://doc.rust-lang.org/cargo/reference/workspace.html#the-lints-table), [resolver 3](https://doc.rust-lang.org/cargo/reference/resolver.html#resolver-versions)):

```toml
[workspace]
members = ["crates/*"]
resolver = "3"

[workspace.package]
edition = "2024"
rust-version = "1.100"
publish = false

[workspace.lints.rust]
unsafe_code = "forbid"
unreachable_pub = "warn"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"
missing_errors_doc = "allow"
missing_panics_doc = "allow"
must_use_candidate = "allow"
return_self_not_must_use = "allow"
needless_pass_by_value = "allow"
unwrap_used = "warn"
expect_used = "warn"
panic = "warn"
todo = "warn"
unimplemented = "warn"
dbg_macro = "warn"
string_slice = "warn"
clone_on_ref_ptr = "warn"
mod_module_files = "warn"
allow_attributes = "warn"
```

Every crate: `[lints] workspace = true`, `edition.workspace = true`, `rust-version.workspace = true`.

`clippy.toml` ([config](https://doc.rust-lang.org/clippy/configuration.html)):

```toml
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-panic-in-tests = true
disallowed-methods = [{ path = "std::thread::sleep", reason = "use tokio::time::sleep" }]
```

CI, in this order, all with `--locked`:

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo sqlx prepare --workspace --check -- --all-targets
cargo test --workspace --locked
cargo deny check
```

- Lints are `warn` in config, `-D warnings` only in CI, so local iteration is not blocked.
- `clippy.toml` `allow-*-in-tests` covers `#[test]` fns and `#[cfg(test)]` modules, not helper fns in `tests/*.rs`; return `Result<_, Box<dyn Error>>` there and use `?`.
- Do not enable `clippy::nursery` (unstable, conflicts with `pub(crate)`), `clippy::cargo` (noise; cargo-deny covers it), or the whole `restriction` group ([lint groups](https://doc.rust-lang.org/clippy/lints.html)).
- Pedantic stays on: the allows above are the only exceptions; add another only with a one-line reason in the PR.
- `needless_pass_by_value` is allowed because axum extractors are taken by value.
- Suppress with `#[expect(lint, reason = "...")]`, never `#[allow]` ([expect](https://doc.rust-lang.org/reference/attributes/diagnostics.html#the-expect-attribute)); a stale `expect` warns.
- Bump the toolchain in its own PR; `rust-version` equals the pinned minor (this is an application, no MSRV promise).

## Editions and language

- Edition 2024 everywhere ([guide](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)).
- Use let chains (`if let Some(x) = a && x.ok()`), stable in 2024 since 1.88 ([release](https://blog.rust-lang.org/2025/06/26/Rust-1.88.0/)).
- Use native `async fn` in traits and async closures ([1.85](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/)); no `async-trait` unless `dyn` dispatch is required.
- Use `let ... else` for early exits; `?` over `match` on `Result`/`Option`.
- Do not use `_ =>` on enums you own; list the variants so new ones break the build.
- Numeric conversions: `From`/`TryFrom`, not `as` (pedantic `cast_*` lints enforce).
- Index strings with `.get(a..b)`, never `&s[a..b]`: arbitrary UTF-8 note text panics on a char boundary (`string_slice`).
- Iterator chains when they read linearly; `for` when mutation or early exit is the point; `Arc::clone(&x)` not `x.clone()`.
- Parameters take `&str`, `&[T]`, `&Path`, not `&String`, `&Vec<T>`, `&PathBuf`; use `impl Into<String>` only on constructors.
- `macro_rules!` only when it removes real repetition that a function or generic cannot.

## Naming

Follows [API Guidelines naming](https://rust-lang.github.io/api-guidelines/naming.html); the project-relevant parts:

- Casing per [C-CASE](https://rust-lang.github.io/api-guidelines/naming.html#c-case): `snake_case` fns/modules, `UpperCamelCase` types, `SCREAMING_SNAKE_CASE` consts; acronyms are one word (`Http`, `OAuth`, `Sha256`).
- Conversions per [C-CONV](https://rust-lang.github.io/api-guidelines/naming.html#c-conv): `as_` borrowed to borrowed and free, `to_` expensive or owned result, `into_` consumes self.
- Getters have no `get_` prefix ([C-GETTER](https://rust-lang.github.io/api-guidelines/naming.html#c-getter)): `page.path()`, not `page.get_path()`; `get` only for keyed lookup returning `Option`.
- Constructors are inherent associated fns: `new`, `with_<x>`, `from_<x>`, `try_new`, per [C-CTOR](https://rust-lang.github.io/api-guidelines/predictability.html#c-ctor).
- Infallible conversion is `impl From<A> for B`; fallible is `TryFrom`; text parsing is `FromStr`; do not write ad hoc `from_str`/`to_x` fns that shadow them.
- Error types are `<Noun><Verb>Error` or `<Domain>Error` ([C-WORD-ORDER](https://rust-lang.github.io/api-guidelines/naming.html#c-word-order)); variants have no `Error` suffix.
- No `_async`, `_impl`, `_helper`, `Manager`, `Util` names; `async fn` is already visible.
- Full words; abbreviations only when universal (`id`, `tx`, `db`, `req`); `config`, not `cfg`.
- Newtype every identifier and unit: `PageId(i64)`, not bare `i64` ([C-NEWTYPE](https://rust-lang.github.io/api-guidelines/type-safety.html#c-newtype)).
- Use enums, not `bool` or `&str`, for mode arguments ([C-CUSTOM-TYPE](https://rust-lang.github.io/api-guidelines/type-safety.html#c-custom-type)).
- Test names state behavior: `restoring_a_deleted_page_recreates_its_path`.

## Visibility and API surface

- Default to private; widen to `pub(crate)`, then `pub` only when another crate needs it.
- In `openmemory-server` and `openmemory-cli` (binaries) nothing is `pub` below the crate root; `unreachable_pub` enforces it.
- `openmemory-ops` exposes a small deliberate API from `lib.rs` (`pub use` of the few types); internals stay `pub(crate)`.
- Struct fields private by default ([C-STRUCT-PRIVATE](https://rust-lang.github.io/api-guidelines/future-proofing.html#c-struct-private)); public fields only on plain data records with no invariants.
- Enforce invariants in constructors (`Path::parse`) so invalid values cannot exist; do not re-validate downstream.
- `#[derive(Debug)]` on every type; hand-write `Debug` for secrets (tokens, keys, DB URL) so they print `[redacted]`.
- Derive order: `Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default`, then serde/sqlx/schemars.

## Modules and layout

- File-named modules only: `pages.rs` + `pages/queries.rs`; no `mod.rs` ([modules](https://doc.rust-lang.org/reference/items/modules.html); `mod_module_files` enforces).
- Organize by domain, not by layer: `pages/`, `files/`, `oauth/`, `jobs/`; each owns its routes, handlers, queries, and types. No top-level `handlers/`, `models/`, `utils/`.
- One main concept per file, named after it (`page_path.rs` holds `PagePath`).
- Soft cap 300 lines per file; when a file passes it, split the module. Never add divider comments.
- `lib.rs` and `main.rs` only declare modules, re-export, and wire; `main` parses config, inits tracing, calls `run().await`.
- Dependency direction: `openmemory-cli -> openmemory-ops <- openmemory-server`; the binaries never depend on each other.
- `openmemory-ops` is pure logic: no `sqlx`, `axum`, or runtime-coupled types; it must compile and test without a database.
- Import by `crate::` path, not `super::super::`; `use super::*` only inside `#[cfg(test)] mod tests`.
- Let rustfmt sort imports; do not hand-group them.

## Errors

See [C-GOOD-ERR](https://rust-lang.github.io/api-guidelines/interoperability.html#c-good-err), [thiserror](https://docs.rs/thiserror), [anyhow](https://docs.rs/anyhow).

- `openmemory-ops` and every module of `openmemory-server`: typed errors with `thiserror`; one enum per module, variants named by cause.
- `anyhow` only in binary entry code (`main`, CLI command runners, startup wiring); never in a signature of `openmemory-ops` or a handler.
- `openmemory-server` has one `AppError` implementing `IntoResponse`; handlers return `Result<T, AppError>` ([axum errors](https://docs.rs/axum/latest/axum/error_handling/index.html)).
- Map internal errors to status codes in one place; the response body never contains SQL, paths, or `source` chains.
- Error messages are lowercase, no trailing period, no wrapped cause text (the cause is `#[source]`).
- `#[from]` only for a lossless 1:1 wrap; otherwise map explicitly so context is not lost.
- `anyhow::Context` adds the operation (`.context("read credentials file")`), not the cause.
- Log an error or return it, never both; the boundary (`AppError::into_response`, `main`) logs.
- Non-test code has no `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `dbg!` (lints above).
- A provably infallible case is expressed in types, not `expect`; if truly unavoidable, `#[expect(clippy::expect_used, reason = "...")]`.
- Parse, don't validate: turn input into a typed value at the edge, return `Err` on bad input.

## Async and tokio

See the [tokio tutorial](https://tokio.rs/tokio/tutorial) and [what is blocking](https://ryhl.io/blog/async-what-is-blocking/).

- `#[tokio::main]` and `#[tokio::test]` only in binaries and tests; `openmemory-ops` has no runtime.
- Never block a worker: no `std::fs`, `std::thread::sleep`, sync `Command`, or loops over 100 microseconds without an `.await`.
- Blocking or CPU-heavy work goes through [`spawn_blocking`](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html); files via `tokio::fs`.
- External processes (`pdftotext`): `tokio::process::Command` with `kill_on_drop(true)` inside `tokio::time::timeout`.
- Never hold a `std::sync::Mutex` guard across `.await`; keep the critical section tiny. `tokio::sync::Mutex` only when the lock must span an `.await` ([shared state](https://tokio.rs/tokio/tutorial/shared-state)).
- Prefer message passing or immutable `Arc` state over locks; no global `static` state.
- Channels are bounded; an unbounded channel needs a stated reason.
- Every `tokio::spawn` is owned: tracked in a `JoinSet` or `TaskTracker`, its result handled, stopped by a `CancellationToken`.
- Shutdown: `axum::serve(..).with_graceful_shutdown(signal)` then drain jobs ([shutdown](https://tokio.rs/tokio/topics/shutdown)).
- In `select!` branches be cancellation-safe; do not put non-cancel-safe futures (partial reads, multi-step writes) in them ([select](https://docs.rs/tokio/latest/tokio/macro.select.html)).
- Every outbound call and request has a timeout; set body limits (`DefaultBodyLimit`) and request timeouts (`tower_http::timeout`).

## HTTP, MCP, and glue (axum, rmcp, clap)

- Handlers are thin: extract, call a service fn, map the result. No SQL and no business rules in handlers.
- REST handlers and MCP tools call the same service functions; tool glue has no logic of its own.
- Each domain module exposes `fn router() -> Router<AppState>`; `main` merges them.
- `AppState` is `Clone` with `Arc` fields; extract with `State<AppState>`; body-consuming extractor goes last ([order](https://docs.rs/axum/latest/axum/extract/index.html#the-order-of-extractors)).
- Request/response/tool-parameter types are separate from DB row types and from `openmemory-ops` types; convert with `From`.
- Text that LLMs or API consumers read (MCP tool descriptions, OpenAPI summaries) is data: put it in the macro attribute (`description = "..."`), not in `///`.
- clap derive only: one `Cli` struct, one `Command` enum, one fn per subcommand; `main` returns `anyhow::Result<()>` once it can fail (clippy `unnecessary_wraps` rejects it before).
- Server config comes from clap `#[arg(env = "...")]`; secrets are wrapped in a redacting newtype.
- CLI output goes to stdout, diagnostics to stderr; the server and `openmemory-ops` never print, they use `tracing`.
- Object storage is built once and injected as `Arc<dyn ObjectStore>`; tests use `object_store::memory::InMemory`.

## Security

- Compare tokens and hashes with `subtle::ConstantTimeEq`; store only hashes of tokens, codes, and secrets.
- SQL is always parameterized; LIKE input is escaped before use.
- Never log request bodies, note content, tokens, cookies, or credentials.
- `unsafe` is forbidden workspace-wide; lifting it needs a recorded decision, a dedicated crate, and `// SAFETY:` on each block (a WHY comment, so allowed).
- Don't hand-roll crypto or WebAuthn; use `webauthn-rs` and audited primitives.

## SQL (sqlx)

See [`query!`](https://docs.rs/sqlx/latest/sqlx/macro.query.html), [offline mode](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md#enable-building-in-offline-mode-with-query).

- Use `query!`, `query_as!`, `query_scalar!` (compile-time checked) by default; unchecked `query`/`QueryBuilder` only for dynamic SQL (the `where` grammar), always with `push_bind`.
- Never build SQL with `format!` or string concatenation.
- Commit `.sqlx/` (`cargo sqlx prepare --workspace -- --all-targets`) in the same PR as the query change; CI runs `--check`; builds use `SQLX_OFFLINE=true`.
- Only the module that owns a table touches it; queries live in that module's `queries.rs`; row structs are private and mapped to domain types.
- Name columns; no `select *`. Fix nullability with `as "col!"` / `as "col?"` and types with `as "id: PageId"`; newtypes derive `sqlx::Type` with `#[sqlx(transparent)]`.
- SQL is inline in the macro and follows `sql.md` (uppercase keywords, one clause per line).
- Functions take `&mut PgConnection` (or `impl PgExecutor`) so they compose inside a transaction; only service-level fns take `&PgPool`.
- Transactions: `let mut tx = pool.begin().await?; ... tx.commit().await?;`; dropping rolls back; no `.await` of foreign I/O (network, blob store) inside a transaction.
- Migrations: `migrations/` with `sqlx::migrate!`, forward-only, never edited after merge, one concern per file.
- One `PgPool`, built once with explicit `max_connections` and `acquire_timeout`.
- Pick one date-time crate for the workspace (`time`) and enable only that sqlx feature.

## Logging (tracing)

See [tracing](https://docs.rs/tracing/latest/tracing/) and [`#[instrument]`](https://docs.rs/tracing/latest/tracing/attr.instrument.html).

- Structured fields, constant message: `info!(page_id = %id, version, "page restored")`, never `info!("restored {id}")`.
- `%` for Display, `?` for Debug; errors as `error = %err`.
- `#[instrument(skip_all, fields(page_id = %id))]` on service fns that cross I/O; always `skip_all`, then opt fields in.
- Levels: `error` needs a human, `warn` is degraded but handled, `info` is one line per significant state change, `debug`/`trace` for diagnostics.
- Subscriber and `EnvFilter` are initialized only in binaries; libraries only emit events.
- Request-level spans come from `tower_http::trace::TraceLayer`; do not log per request by hand.

## Testing

See [test organization](https://doc.rust-lang.org/book/ch11-03-test-organization.html), [`#[sqlx::test]`](https://docs.rs/sqlx/latest/sqlx/attr.test.html).

- Unit tests live in the file under `#[cfg(test)] mod tests`; if they pass ~100 lines, move them to a sibling `<name>/tests.rs`.
- Integration tests in `crates/<crate>/tests/`, black-box through the public API or the `Router`.
- DB tests use `#[sqlx::test]` (fresh database per test, migrations applied); never share state between tests.
- Drive the app with `tower::ServiceExt::oneshot` on the `Router` ([axum example](https://github.com/tokio-rs/axum/blob/main/examples/testing/src/main.rs)); no real port binding.
- `openmemory-ops` tests are pure and need no database; cover the grammar and diff/merge logic with table-driven cases.
- Prefer real Postgres and `InMemory` object store over mocks; add a trait seam only for a real second implementation.
- Tests may `unwrap`/`expect`; assert on values and error variants, not message strings.

## Dependencies

- Declare every version once in root `[workspace.dependencies]`, sorted; members use `dep.workspace = true`; internal crates are listed there by `path`.
- Workspace entries set `default-features = false` and list only needed features; no `features = ["full"]` (tokio included).
- A member may add features to a workspace dep, never re-declare the version.
- New dependency needs a one-line reason in the PR; prefer std or a few lines of code over a crate for trivial needs.
- One crate per job (one HTTP client, one date-time, one serializer for config).
- Commit `Cargo.lock`; CI uses `--locked`.
- `cargo deny check` gates advisories, licenses, duplicate majors, and unknown registries ([cargo-deny](https://embarkstudios.github.io/cargo-deny/)).

## Comments

Default: none. The code (names, types, small functions) says what and how.

A comment is allowed only when the code cannot express **why**: a protocol quirk, a non-obvious invariant, a workaround for a named defect. One short line, above the line it explains, never restating the code.

- No `///` or `//!` doc comments, no section dividers (`// ---- queries ----`; split the module), no commented-out code, no `TODO` (open an issue).
- `// SAFETY:` and `#[expect(..., reason = "...")]` follow the same why-only rule.
- If a comment explains what code does, rename or extract instead.

Good:

```rust
// RFC 9728: the metadata URL must be echoed in WWW-Authenticate, or clients stop retrying.
let challenge = format!(r#"Bearer resource_metadata="{url}""#);
```

Bad:

```rust
// Fetch the page from the database
let page = pages::find(&mut conn, id).await?;
```

## Writing style

- English only: identifiers, messages, commits, tests.
- Small functions with one level of abstraction; a function over ~60 lines or 5 parameters is split (`too_many_lines` fires at 100, `too_many_arguments` at 7).
- Make illegal states unrepresentable: sum types over flag combinations, `Option` only for real absence.
- Add nothing speculative: no unused generics, config knobs, feature flags, or trait seams.

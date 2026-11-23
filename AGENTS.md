# Agents

Read before changing code:

- `docs/operations.md`: how it is installed, configured, backed up and developed.
- `docs/style/rust.md`, `docs/style/typescript.md`, `docs/style/sql.md`: how code is written.

Rules:

- Code explains itself. Write no comments, except one short line where the code cannot say why (a protocol quirk, a non-obvious invariant, a workaround). Never restate what the code does. No doc comments, no divider comments, no commented-out code.
- English everywhere: code, messages, commits, docs.
- Add nothing the product does not need. No speculative options, flags or abstractions.
- Every change passes the CI checks in `.github/workflows/ci.yml` before it is done.

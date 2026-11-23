# memory

Self-hosted memory shared by people and their AI agents.

People and agents read and write the same pages. Every change is recorded with its author and can be reverted. Originals such as PDFs sit beside the pages that cite them.

- Pages: markdown with fields, current truth and an append-only timeline
- Agents: Claude Code, Claude, ChatGPT, Codex and Cursor over MCP, plus the `memory` CLI
- People: web app with passkey sign-in
- Search: Korean and English, over pages and PDF text
- Backups: continuous, encrypted, with point-in-time restore

## Run

Needs Docker with Compose and an HTTPS origin in front of the server.

```
git clone https://github.com/minodevss/memory
cd memory
cp .env.example .env
docker compose up -d
docker compose logs server | grep bootstrap
```

Set `MEMORY_PUBLIC_URL`, `MEMORY_DB_PASSWORD` and `MEMORY_BACKUP_PASSPHRASE` in `.env` first. Open the logged link to create the first account.

Images: `ghcr.io/minodevss/memory` and `ghcr.io/minodevss/memory-postgres`.

## Connect an agent

The MCP endpoint is `<MEMORY_PUBLIC_URL>/mcp`. Each connection is approved in the browser with a passkey.

```
curl -fsSL https://raw.githubusercontent.com/minodevss/memory/main/install.sh | sh
memory login https://memory.example.com
```

The script installs the latest release to `~/.local/bin`. Set `MEMORY_VERSION` for a fixed tag, `MEMORY_INSTALL_DIR` for another directory, or `MEMORY_DOWNLOAD_URL` for a mirror.

Claude, ChatGPT, Codex and Cursor take the same URL as a custom connector. The home screen lists the exact steps for each.

## Docs

- [Operations](docs/operations.md): configuration, backup, restore, upgrade, development
- [Style](docs/style): Rust, TypeScript and SQL

## License

[Apache-2.0](LICENSE)

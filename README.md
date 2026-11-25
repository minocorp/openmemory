# OpenMemory

Self-hosted memory shared by people and their AI agents.

People and agents read and write the same pages. Every change is recorded with its author and can be reverted. Originals such as PDFs sit beside the pages that cite them.

- Pages: markdown with fields, current truth and an append-only timeline
- Agents: Claude Code, Claude, ChatGPT, Codex and Cursor over MCP
- People: web app with passkey sign-in, and the `openmemory` CLI
- Search: Korean and English, over pages and PDF text
- Backups: continuous, encrypted, with point-in-time restore

## Run

Needs Docker with Compose and an HTTPS origin in front of the server.

```
git clone https://github.com/minocorp/openmemory
cd openmemory
cp .env.example .env
docker compose up -d
docker compose logs server | grep bootstrap
```

Set `OPENMEMORY_PUBLIC_URL`, `OPENMEMORY_DB_PASSWORD` and `OPENMEMORY_BACKUP_PASSPHRASE` in `.env` first. Open the logged link to create the first account.

Images: `ghcr.io/minocorp/openmemory` and `ghcr.io/minocorp/openmemory-postgres`.

## Connect an agent

The MCP endpoint is `<OPENMEMORY_PUBLIC_URL>/mcp`. Each connection is approved in the browser with a passkey. Claude, ChatGPT, Codex and Cursor take the URL as a custom connector. The home screen lists the exact steps for each.

## CLI

```
curl -fsSL https://raw.githubusercontent.com/minocorp/openmemory/main/install.sh | sh
openmemory auth login --host https://openmemory.example.com
openmemory page list
openmemory page view clients/acme
openmemory page append clients/acme "Called back" --source phone
openmemory page edit clients/acme
openmemory search renewal --where status=active
openmemory file upload ./contract.pdf clients/acme/contract.pdf
```

The script installs the latest release to `~/.local/bin`. Set `OPENMEMORY_VERSION` for a fixed tag, `OPENMEMORY_INSTALL_DIR` for another directory, or `OPENMEMORY_DOWNLOAD_URL` for a mirror. `openmemory auth login` opens the browser; add `--device` on a machine without one. Output is JSON when stdout is not a terminal or with `--json`. `openmemory api /api/tree` makes any authenticated REST call, and `openmemory completion zsh` prints shell completions.

For CI create a personal access token in Settings, then set `OPENMEMORY_HOST` and `OPENMEMORY_TOKEN`. They override stored credentials.

## Docs

- [Operations](docs/operations.md): configuration, backup, restore, upgrade, development
- [Style](docs/style): Rust, TypeScript and SQL

## License

[Apache-2.0](LICENSE)

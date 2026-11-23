# Operations

## Install

Needs Docker with Compose. The server speaks plain HTTP only. Put your own HTTPS reverse proxy in front: claude.ai, ChatGPT and passkeys need a public HTTPS origin, and `MEMORY_PUBLIC_URL` must be that origin. Passkeys are bound to its domain.

```
cp .env.example .env
docker compose up -d
docker compose logs server | grep bootstrap
```

Fill `.env` first. Use letters and digits in `MEMORY_DB_PASSWORD`. The log line holds the one-time link that creates the first admin.

The server listens on `MEMORY_HTTP_BIND` (default `127.0.0.1:8080`). If your proxy is not on the host, bind another address and list the proxy in `MEMORY_TRUSTED_PROXIES`. The rate limiter then reads `X-Forwarded-For` from those peers only; otherwise it uses the socket peer.

## Environment

| Variable | Meaning | Default |
|---|---|---|
| `MEMORY_PUBLIC_URL` | Public HTTPS origin | required |
| `MEMORY_DB_PASSWORD` | Postgres password | required |
| `MEMORY_BACKUP_PASSPHRASE` | Encrypts every backup. Losing it loses the backups | required |
| `MEMORY_HTTP_BIND` | Host address and port for the server | `127.0.0.1:8080` |
| `MEMORY_TRUSTED_PROXIES` | Comma-separated addresses or CIDRs allowed to set `X-Forwarded-For` | none |
| `MEMORY_TIMEZONE` | IANA time zone for dates | `UTC` |
| `MEMORY_MAX_UPLOAD_BYTES` | Upload size cap | 52428800 |
| `MEMORY_BACKUP_RETENTION_DAYS` | Days of backups and blob snapshots kept | 14 |
| `MEMORY_BACKUP_S3_ENDPOINT`, `_BUCKET`, `_REGION`, `_ACCESS_KEY`, `_SECRET_KEY` | Off-host backup repository (S3-compatible, https) | none |
| `MEMORY_BACKUP_S3_URI_STYLE` | `host` or `path` | `host` |
| `MEMORY_S3_ENDPOINT`, `_BUCKET`, `_REGION`, `_ACCESS_KEY`, `_SECRET_KEY` | Store blobs in S3 instead of the `blobs` volume | none |

## Backup

The `pgbackrest` container takes a full backup weekly, an incremental daily, checks WAL archiving every 5 minutes and snapshots the `blobs` volume hourly with restic. Without `MEMORY_BACKUP_S3_*` the repository is the local `backup` volume, which does not survive losing the host: set an S3 target. With one set, every backup goes to both repositories.

Blobs kept in an S3 store (`MEMORY_S3_*`) are not copied. Enable versioning on that bucket.

`GET /healthz` reports database, backup age, WAL archive age, off-host flag and the last restore test, with `status: degraded` when any is stale or failed.

## Restore

```
scripts/restore.sh
scripts/restore.sh --time '2026-11-23 10:00:00+00'
scripts/restore.sh --repo 2
```

It stops the stack, restores the database from the latest backup (or to the given time) and the latest blob snapshot, then starts the stack. `--repo 2` restores from the S3 repository, as on a new host with empty volumes and the same `.env`.

## Restore test

The backup container runs it weekly. To run it now:

```
docker compose exec pgbackrest memory-restore-test
```

It restores into a throwaway Postgres, replays WAL, and passes when the restored page count lies between the live counts before and after. The result appears in `/healthz`.

## Upgrade

```
git pull
docker compose pull
docker compose up -d
```

Migrations run at startup. Back up first with `docker compose exec pgbackrest pgbackrest --type=full backup`.

## Development

```
docker compose -f compose.dev.yml up -d
export DATABASE_URL=postgres://memory:memory@127.0.0.1:5441/memory
cargo run -p memory-server
```

`compose.dev.yml` runs Postgres only, on `127.0.0.1:5441`. `sqlx` macros and `#[sqlx::test]` read `DATABASE_URL`. After changing a query run `cargo sqlx prepare --workspace -- --all-targets`.

### Web

```
MEMORY_PUBLIC_URL=http://localhost:5173 cargo run -p memory-server
cd web && pnpm install && pnpm dev
```

Open `http://localhost:5173`, not the server port. Vite proxies `/api`, `/mcp`, `/oauth`, `/.well-known` and `/healthz` to `http://localhost:8080` (`MEMORY_API_ORIGIN` changes the target). The server checks the `Origin` of every non-GET request against `MEMORY_PUBLIC_URL`, and passkeys are bound to its host, so the public URL must be the Vite origin. The first start prints the bootstrap link with that origin.

After a server change that touches the API, regenerate the web types with `pnpm gen:api`.

### End-to-end tests

```
docker compose -f compose.dev.yml up -d
cd web && pnpm exec playwright install chromium && pnpm e2e
```

`pnpm e2e` needs the dev Postgres above, Rust and Chromium. It recreates the database `memory_e2e` in it, builds and starts `memory-server` on `127.0.0.1:8081` with a bootstrap link, starts Vite on `localhost:4173` and drives Chromium with a virtual WebAuthn authenticator. Screenshots of each screen land in `run/ui/`. The server runs with `MEMORY_TRUSTED_PROXIES=127.0.0.1,::1` and every browser context sends its own `X-Forwarded-For`, so the per-IP rate limits do not couple tests. `E2E_EMBEDDED=1` after `pnpm build` serves the SPA from `memory-server` itself instead of Vite, which is how `60-csp.spec.ts` checks the content security policy. `E2E_INSTANCE=n` moves the web and API ports, the database and the blob directory so several runs can go in parallel; pass `--output=<dir>` to keep their `test-results` apart.

## Release

Set `version` in the root `Cargo.toml`, merge, then publish a GitHub release tagged `v<version>`. The release workflow refuses a tag that does not match and attaches the CLI binaries and images.

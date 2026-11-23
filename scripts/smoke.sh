#!/bin/sh
set -eu

project=${MEMORY_SMOKE_PROJECT:-memory-smoke}
env_file=$(mktemp)
trap 'docker compose -p "$project" --env-file "$env_file" down -v; rm -f "$env_file"' EXIT
cat >"$env_file" <<EOF
MEMORY_PUBLIC_URL=http://localhost:18080
MEMORY_DB_PASSWORD=smoke
MEMORY_BACKUP_PASSPHRASE=smoke-passphrase
MEMORY_HTTP_BIND=127.0.0.1:18080
EOF
compose() { docker compose -p "$project" --env-file "$env_file" "$@"; }

compose up -d --build --wait --wait-timeout 300
compose logs server | grep -q "open this link to create the first admin"
curl -fsS http://localhost:18080/healthz

until compose exec -T pgbackrest test -s /backup/status.json; do sleep 5; done
until compose exec -T pgbackrest sh -c 'pgbackrest info --output=json | jq -e ".[0].backup | length > 0"' >/dev/null; do sleep 5; done
compose exec -T pgbackrest memory-restore-test
compose exec -T pgbackrest cat /backup/state/restore-test.json
until curl -fsS http://localhost:18080/healthz | grep -q '"status":"ok"'; do sleep 5; done
curl -fsS http://localhost:18080/healthz

#!/bin/sh
set -eu

project=${OPENMEMORY_SMOKE_PROJECT:-openmemory-smoke}
env_file=$(mktemp)
trap 'docker compose -p "$project" --env-file "$env_file" down -v; rm -f "$env_file"' EXIT
cat >"$env_file" <<EOF
OPENMEMORY_PUBLIC_URL=http://localhost:18080
OPENMEMORY_DB_PASSWORD=smoke
OPENMEMORY_BACKUP_PASSPHRASE=smoke-passphrase
OPENMEMORY_HTTP_BIND=127.0.0.1:18080
EOF
compose() { docker compose -p "$project" --env-file "$env_file" "$@"; }

compose up -d --build --wait --wait-timeout 300
compose logs server | grep -q "open this link to create the first admin"
curl -fsS http://localhost:18080/healthz

until compose exec -T pgbackrest test -s /backup/status.json; do sleep 5; done
until compose exec -T pgbackrest sh -c 'pgbackrest info --output=json | jq -e ".[0].backup | length > 0"' >/dev/null; do sleep 5; done
compose exec -T pgbackrest openmemory-restore-test
compose exec -T pgbackrest cat /backup/state/restore-test.json
until curl -fsS http://localhost:18080/healthz | grep -q '"status":"ok"'; do sleep 5; done
curl -fsS http://localhost:18080/healthz

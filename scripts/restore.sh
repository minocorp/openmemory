#!/bin/sh
set -eu

docker compose stop server pgbackrest postgres
docker compose run --rm --no-deps restore openmemory-restore "$@"
docker compose up -d

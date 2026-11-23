#!/bin/sh
set -eu

docker compose stop server pgbackrest postgres
docker compose run --rm --no-deps restore memory-restore "$@"
docker compose up -d

#!/bin/bash

set -euo pipefail

source docker/source.sh
export PLURALSYNC_STAGE=local
export FRONTEND_DIST=./../frontend/dist
export PATH_TO_PLURALSYNC_API_EXEC=../target/debug/pluralsync
export PATH_TO_PLURALSYNC_GLOBAL_MANAGER_EXEC=../target/debug/pluralsync-global-manager

./docker/stop.sh local || true

docker compose -f docker/docker.compose.yml pull
docker compose -f docker/docker.compose.yml up pluralsync-db -d

await pluralsync-db "listening on IPv4 address"

export DATABASE_URL="postgres://postgres:postgres@localhost:5432/pluralsync"

( cd docker && cargo sqlx migrate run )

rm -v .sqlx/*.json || true

./steps/12-backend-cargo-build.sh

cargo sqlx prepare -- --lib --tests

./docker/stop.sh local

unset DATABASE_URL

# this build should use the prepared queries now
./steps/12-backend-cargo-build.sh

echo "Refreshed SQLx prepare."

#!/bin/bash

set -eo pipefail

# This script runs Rust integration tests that require a database connection.
# It starts a PostgreSQL container, runs migrations, and executes cargo test
# with DATABASE_URL set.

source docker/source.sh

export PLURALSYNC_STAGE=local

# Source only the DATABASE_URL from local.env (avoid unbound variable issues)
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/pluralsync"
export POSTGRES_PASSWORD=postgres

echo "=== Running Rust Database Integration Tests ==="

./docker/stop.sh "$PLURALSYNC_STAGE" || true

docker compose -f docker/docker.compose.yml up pluralsync-db -d

await pluralsync-db "listening on IPv4 address"

# Run migrations with explicit DATABASE_URL from the docker directory
# Note: export is needed so the subshell inherits the variable
export DATABASE_URL
( cd docker && cargo sqlx migrate run )

cargo test --lib

./docker/stop.sh "$PLURALSYNC_STAGE"

echo "✅ Rust Database Integration Tests completed."

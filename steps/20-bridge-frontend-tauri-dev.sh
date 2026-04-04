#!/bin/bash

set -euo pipefail

export TAURI_APP_PATH="bridge-src-tauri"
cp bridge-src-tauri/{template.,}tauri.conf.json

export PLURALSYNC_BASE_URL="http://localhost:8080"

echo "PLURALSYNC_BASE_URL: $PLURALSYNC_BASE_URL"
echo "IF you want to send requsts against a backend,"
echo "ensure that the backend is running via source secrets + ./test/start-backend-for-bridge-frontend.sh"

echo ""
echo "ATTENTION! global config file is being used!"


force_vite() {
    sleep 1.5s
    echo "Forcing vite to render..."
    curl --no-progress-meter http://localhost:5173 >/dev/null
    echo "Ok."
}


force_vite &
cargo tauri dev


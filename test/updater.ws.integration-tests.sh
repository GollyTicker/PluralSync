#!/bin/bash

set -euo pipefail

export ENABLE_DISCORD_STATUS_MESSAGE=true
ENABLE_DISCORD_STATUS_MESSAGE=true
ENABLE_VRCHAT=false
ENABLE_DISCORD=true
ENABLE_WEBSITE=false
ENABLE_TO_PLURALKIT=false
ENABLE_FROM_WEBSOCKET=true
ENABLE_FROM_PLURALKIT=false
ENABLE_FROM_SP=false

source ./test/source.sh
source ./test/plural_system_to_test.sh

ws_open() {
    websocat --log-no-ts "ws://localhost:8080/api/user/platform/pluralsync/events" &
    WS_PID=$!
    WS_STDIN="/proc/$WS_PID/fd/0"
    WS_STDOUT="/proc/$WS_PID/fd/1"
    sleep 0.3
}

ws_close() {
    if [[ -n "${WS_PID:-}" ]]; then
        kill "$WS_PID" 2>/dev/null || true
        wait "$WS_PID" 2>/dev/null || true
        WS_PID=""
        WS_STDIN=""
        WS_STDOUT=""
    fi
}

ws_send() {
    echo "$1" > "$WS_STDIN"
}

ws_receive() {
    TIMEOUT="${1:-5}"

    if [[ ! -e "/proc/$WS_PID" ]]; then
        echo ""
        return 0
    fi

    echo "$(timeout "${TIMEOUT}s" head -n 1 < "$WS_STDOUT" 2>/dev/null)" || true
}

ws_status() {
    if [[ -z "${WS_PID:-}" ]]; then
        echo "closed"
        return 0
    fi

    if kill -0 "$WS_PID" 2>/dev/null; then
        echo "open"
    else
        echo "closed"
    fi
}


check_discord_status_contains() {
    EXPECTED="$1"
    RESPONSE="$(curl -s "https://discord.com/api/v10/users/@me/settings" -H "Authorization: $DISCORD_STATUS_MESSAGE_TOKEN")"
    STATUS="$(echo "$RESPONSE" | jq -r .custom_status.text)"
    echo "$STATUS" | grep -q "$EXPECTED"
}

check_discord_status_not_contains() {
    UNEXPECTED="$1"
    RESPONSE="$(curl -s "https://discord.com/api/v10/users/@me/settings" -H "Authorization: $DISCORD_STATUS_MESSAGE_TOKEN")"
    STATUS="$(echo "$RESPONSE" | jq -r .custom_status.text)"
    echo "$STATUS" | grep -qv "$UNEXPECTED"
}

test_invalid_login_closes_connection() {
    ws_open
    ws_send '{"type":"login","user":"test@example.com","auth":"invalid.token.here"}'
    ws_receive | grep -q '"type":"error"'
    [[ "$(ws_status)" == "closed" ]]
    echo "✅ invalid_login_closes_connection"
}

test_valid_login_and_fronters() {
}

test_initial_fronting_persisted_and_forwarded() {
}

test_fronting_updates_forwarded() {
}

test_invalid_fronting_data_keeps_connection() {
}

test_ping_pong() {
}

test_websocket_restart_pushes_updates() {
}

test_status_persisted_across_reboot() {
}

test_empty_fronters_clears_all() {
}

export BASE_URL="http://localhost:8080"

start_updater() {
    ./docker/start.sh local > docker/logs/start.log 2>&1
    setup_test_user
    await pluralsync-api "Rocket has launched from"
}

stop_updater() {
    ./docker/stop.sh local > docker/logs/stop.log 2>&1
}

main() {
    stop_updater
    ./steps/12-backend-cargo-build.sh

    if ! command -v websocat &>/dev/null; then
        echo "ERROR: websocat is not installed."
        exit 1
    fi

    start_updater

    test_invalid_login_closes_connection
    test_valid_login_and_fronters
    test_initial_fronting_persisted_and_forwarded
    test_fronting_updates_forwarded
    test_invalid_fronting_data_keeps_connection
    test_ping_pong
    test_websocket_restart_pushes_updates
    test_status_persisted_across_reboot
    test_empty_fronters_clears_all

    clear_all_fronts
    echo "✅✅✅ WebSocket Integration Test ✅✅✅"
}

trap stop_updater EXIT

main

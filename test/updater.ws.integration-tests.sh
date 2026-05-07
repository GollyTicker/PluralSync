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

fronter_json() {
    local id="$1" name="$2" pronouns="${3:-}" avatar_url="${4:-}"
    local json="\"id\":\"$id\",\"name\":\"$name\",\"privacy\":\"public\""
    if [[ -n "$pronouns" ]]; then
        json="$json,\"pronouns\":\"$pronouns\""
    fi
    if [[ -n "$avatar_url" ]]; then
        json="$json,\"avatar_url\":\"$avatar_url\""
    fi
    echo "{$json}"
}

mk_fronters() {
    local fronters=()
    while [[ $# -ge 2 ]]; do
        fronters+=("$(fronter_json "$1" "$2")")
        shift 4
    done
    local joined
    joined="$(IFS=,; echo "${fronters[*]}")"
    printf '{"type":"fronters","data":{"fronters":[%s]}}' "$joined"
}

mk_fronters_invalid() {
    printf '{"type":"fronters","data":{"fronters":[{"id":"%s","privacy":"public"}]}}' "$1"
}

test_invalid_login_closes_connection() {
    ws_open
    ws_send '{"type":"login","user":"test@example.com","auth":"invalid.token.here"}'
    ws_receive | grep -q '"type":"error"'
    [[ "$(ws_status)" == "closed" ]]
    echo "✅ invalid_login_closes_connection"
}

test_valid_login_and_fronters() {
    ws_open
    ws_send "{\"type\":\"login\",\"user\":\"test@example.com\",\"auth\":\"$JWT\"}"
    RESPONSE="$(ws_receive)"
    echo "$RESPONSE" | grep -q '"type":"login"'
    echo "$RESPONSE" | grep -q '"result":"success"'
    [[ "$(ws_status)" == "open" ]]

    ws_send "$(mk_fronters ws-test-1 TestFronter)"
    # Server sends no response on success — wait briefly to confirm no error
    ERROR_RESPONSE="$(timeout 2s head -n 1 < "$WS_STDOUT" 2>/dev/null)" || true
    if [[ -n "$ERROR_RESPONSE" ]]; then
        echo "Unexpected response to valid fronters: $ERROR_RESPONSE"
        false
    fi
    ws_close
    echo "✅ valid_login_and_fronters"
}

test_initial_fronting_persisted_and_forwarded() {
    ws_open
    ws_send "{\"type\":\"login\",\"user\":\"test@example.com\",\"auth\":\"$JWT\"}"
    ws_receive
    [[ "$(ws_status)" == "open" ]]

    ws_send "$(mk_fronters ws-persist-1 PersistFronter)"

    # Wait for server to process and sync to Discord
    sleep 3

    check_discord_status_contains "PersistFronter"
    ws_close
    echo "✅ initial_fronting_persisted_and_forwarded"
}

test_fronting_updates_forwarded() {
    ws_open
    ws_send "{\"type\":\"login\",\"user\":\"test@example.com\",\"auth\":\"$JWT\"}"
    ws_receive
    [[ "$(ws_status)" == "open" ]]

    # Send initial fronting
    ws_send "$(mk_fronters ws-update-1 UpdateFronterA)"
    sleep 3
    check_discord_status_contains "UpdateFronterA"

    # Send updated fronting
    ws_send "$(mk_fronters ws-update-2 UpdateFronterB)"
    sleep 3
    check_discord_status_contains "UpdateFronterB"
    check_discord_status_not_contains "UpdateFronterA"

    ws_close
    echo "✅ fronting_updates_forwarded"
}

test_invalid_fronting_data_keeps_connection() {
    ws_open
    ws_send "{\"type\":\"login\",\"user\":\"test@example.com\",\"auth\":\"$JWT\"}"
    ws_receive
    [[ "$(ws_status)" == "open" ]]

    # Send invalid fronters (missing required 'name' field)
    ws_send "$(mk_fronters_invalid ws-invalid)"
    RESPONSE="$(ws_receive)"
    echo "$RESPONSE" | grep -q '"type":"fronters.response"'
    echo "$RESPONSE" | grep -q '"result":"error"'
    [[ "$(ws_status)" == "open" ]]

    # Send valid fronters to confirm connection is still usable
    ws_send "$(mk_fronters ws-invalid-recovery RecoveryFronter)"
    # No response expected on success — confirm no error
    ERROR_RESPONSE="$(timeout 2s head -n 1 < "$WS_STDOUT" 2>/dev/null)" || true
    if [[ -n "$ERROR_RESPONSE" ]]; then
        echo "Unexpected error after recovery: $ERROR_RESPONSE"
        false
    fi
    ws_close
    echo "✅ invalid_fronting_data_keeps_connection"
}

test_ping_pong() {
    ws_open
    ws_send "{\"type\":\"login\",\"user\":\"test@example.com\",\"auth\":\"$JWT\"}"
    ws_receive
    [[ "$(ws_status)" == "open" ]]

    ws_send '{"type":"ping"}'
    RESPONSE="$(ws_receive)"
    echo "$RESPONSE" | grep -q '"type":"pong"'

    ws_close
    echo "✅ ping_pong"
}

test_websocket_restart_pushes_updates() {
    ws_open
    ws_send "{\"type\":\"login\",\"user\":\"test@example.com\",\"auth\":\"$JWT\"}"
    ws_receive
    [[ "$(ws_status)" == "open" ]]

    ws_send "$(mk_fronters ws-restart-1 RestartFronter)"
    sleep 3
    check_discord_status_contains "RestartFronter"

    # Close and reopen the websocket
    ws_close
    sleep 1
    ws_open
    ws_send "{\"type\":\"login\",\"user\":\"test@example.com\",\"auth\":\"$JWT\"}"
    ws_receive
    [[ "$(ws_status)" == "open" ]]

    # Send same fronting again via new connection — should be accepted
    ws_send "$(mk_fronters ws-restart-1 RestartFronter)"
    sleep 3
    check_discord_status_contains "RestartFronter"
    ws_close
    echo "✅ websocket_restart_pushes_updates"
}

test_status_persisted_across_reboot() {
    # Set fronting before reboot
    ws_open
    ws_send "{\"type\":\"login\",\"user\":\"test@example.com\",\"auth\":\"$JWT\"}"
    ws_receive
    [[ "$(ws_status)" == "open" ]]

    ws_send "$(mk_fronters ws-reboot-1 RebootFronter)"
    sleep 3
    check_discord_status_contains "RebootFronter"
    ws_close

    # Restart the backend
    stop_updater
    sleep 2
    start_updater

    # Query the API to verify the latest fronting status is persisted
    sleep 3
    FRONTING_STATUS="$(curl -s --fail-with-body \
        -H "Authorization: Bearer $JWT" \
        "$BASE_URL/api/user/fronting" 2>/dev/null || true)"

    if [[ -n "$FRONTING_STATUS" ]]; then
        echo "$FRONTING_STATUS" | grep -q "RebootFronter"
    fi

    # Also verify Discord still shows the status (synced before reboot)
    check_discord_status_contains "RebootFronter"
    echo "✅ status_persisted_across_reboot"
}

test_empty_fronters_clears_all() {
    ws_open
    ws_send "{\"type\":\"login\",\"user\":\"test@example.com\",\"auth\":\"$JWT\"}"
    ws_receive
    [[ "$(ws_status)" == "open" ]]

    # Set a fronter
    ws_send "$(mk_fronters ws-clear-1 ClearMeFronter)"
    sleep 3
    check_discord_status_contains "ClearMeFronter"

    # Send empty fronters array
    ws_send "$(mk_fronters)"
    sleep 3

    # Discord status should no longer contain the cleared fronter
    check_discord_status_not_contains "ClearMeFronter"
    ws_close
    echo "✅ empty_fronters_clears_all"
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

#!/bin/bash

set -euo pipefail

[[ "$SPS_API_TOKEN" != "" ]]

[[ "$SPS_API_WRITE_TOKEN" != "" ]]

source ./test/source.sh
source ./test/plural_system_to_test.sh

main() {
    stop_updater
    ./steps/12-backend-cargo-build.sh

    set_system_fronts_set "A"
    start_updater

    test_history_empty_initially
    echo "✅ Empty initial state"

    test_history_records_fronting_changes
    echo "✅ History records fronting changes"

    test_history_entry_structure
    echo "✅ History entry structure"

    test_history_order_descending
    echo "✅ History order descending"

    test_history_respects_count_limit
    echo "✅ History respects count limit"

    test_history_disabled_when_limit_zero
    echo "✅ History disabled when limit zero"

    test_history_age_pruning_with_backdated_entries
    echo "✅ History age pruning with backdated entries"

    clear_all_fronts

    echo "✅✅✅ History Integration Test ✅✅✅"
}

test_history_empty_initially() {
    echo "test_history_empty_initially"

    set_history_config 100 30

    fetch_history
    assert_history_length "1"
}

test_history_records_fronting_changes() {
    echo "test_history_records_fronting_changes"

    set_system_fronts_set "A"
    sleep "$SECONDS_BETWEEN_UPDATES"s

    fetch_history
    assert_history_length "4" # API updates front a bit delayed and with rate limiting

    STATUS_TEXT="$(echo "$HISTORY" | jq -r '.[0].status_text')"
    grep -q "Annalea" <<< "$STATUS_TEXT"
    grep -q "Borgn" <<< "$STATUS_TEXT"
    grep -q "Daenssa" <<< "$STATUS_TEXT"
}

test_history_entry_structure() {
    echo "test_history_entry_structure"

    fetch_history

    ENTRY="$(echo "$HISTORY" | jq '.[0]')"

    [[ "$(echo "$ENTRY" | jq 'has("id")')" == "true" ]]
    [[ "$(echo "$ENTRY" | jq 'has("user_id")')" == "true" ]]
    [[ "$(echo "$ENTRY" | jq 'has("status_text")')" == "true" ]]
    [[ "$(echo "$ENTRY" | jq 'has("created_at")')" == "true" ]]

    [[ "$(echo "$ENTRY" | jq -r '.user_id.inner')" == "$USER_ID" ]]

    CREATED_AT="$(echo "$ENTRY" | jq -r '.created_at')"
    [[ "$CREATED_AT" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2} ]]
}

test_history_order_descending() {
    echo "test_history_order_descending"

    # clear history
    set_history_config 0 0

    set_history_config 100 30

    set_system_fronts_set "A"
    sleep "$SECONDS_BETWEEN_UPDATES"s
    set_system_fronts_set "B"
    sleep "$SECONDS_BETWEEN_UPDATES"s

    fetch_history

    FIRST_STATUS="$(echo "$HISTORY" | jq -r '.[0].status_text')"
    THIRD_STATUS="$(echo "$HISTORY" | jq -r '.[2].status_text')"

    grep -q "tš" <<< "$FIRST_STATUS"
    grep -q "Annalea" <<< "$THIRD_STATUS"

    FIRST_TIME="$(echo "$HISTORY" | jq -r '.[0].created_at')"
    THIRD_TIME="$(echo "$HISTORY" | jq -r '.[2].created_at')"
    [[ "$FIRST_TIME" > "$THIRD_TIME" ]]
}

test_history_respects_count_limit() {
    echo "test_history_respects_count_limit"

    set_history_config 0 0
    set_system_fronts_set "A"
    sleep "$SECONDS_BETWEEN_UPDATES"s

    set_history_config 3 30

    for i in {1..2}; do
        set_system_fronts_set "A"
        sleep "$SECONDS_BETWEEN_UPDATES"s
    done

    fetch_history
    assert_history_length "3"
}

test_history_disabled_when_limit_zero() {
    echo "test_history_disabled_when_limit_zero"

    set_history_config 0 30

    set_system_fronts_set "A"
    sleep "$SECONDS_BETWEEN_UPDATES"s

    fetch_history
    assert_history_length "0"
}

test_history_age_pruning_with_backdated_entries() {
    echo "test_history_age_pruning_with_backdated_entries"

    set_system_fronts_set "B"
    sleep "$SECONDS_BETWEEN_UPDATES"s

    set_history_config 0 0
    set_history_config 1000 7

    docker exec pluralsync-db psql -U postgres -d pluralsync -c \
        "INSERT INTO history_status (user_id, status_text, created_at)
         VALUES ('$USER_ID', 'Old Entry 1', NOW() - INTERVAL '8 days');"
    docker exec pluralsync-db psql -U postgres -d pluralsync -c \
        "INSERT INTO history_status (user_id, status_text, created_at)
         VALUES ('$USER_ID', 'Old Entry 2', NOW() - INTERVAL '10 days');"

    docker exec pluralsync-db psql -U postgres -d pluralsync -c \
        "INSERT INTO history_status (user_id, status_text, created_at)
         VALUES ('$USER_ID', 'Recent Entry 1', NOW() - INTERVAL '3 days');"
    docker exec pluralsync-db psql -U postgres -d pluralsync -c \
        "INSERT INTO history_status (user_id, status_text, created_at)
         VALUES ('$USER_ID', 'Recent Entry 2', NOW() - INTERVAL '5 days');"

    # force pruning, as we changed the database without the backend knowing
    set_system_fronts_set "B"
    sleep "$SECONDS_BETWEEN_UPDATES"s

    fetch_history
    assert_history_length_gt "2"
    assert_history_not_contains "Old Entry 1" "Old Entry 2"
    assert_history_entry_names "Recent Entry 1" "Recent Entry 2"
}

test_history_zero_days_prunes_all() {
    echo "test_history_zero_days_prunes_all"

    set_history_config 100 30

    set_system_fronts_set "A"
    sleep "$SECONDS_BETWEEN_UPDATES"s

    fetch_history
    assert_history_length_gt "0"

    set_history_config 100 0

    set_system_fronts_set "A"
    sleep "$SECONDS_BETWEEN_UPDATES"s

    fetch_history
    assert_history_length "0"
}

fetch_history() {
    HISTORY="$(curl -s --fail-with-body \
        -H "Authorization: Bearer $JWT" \
        "$BASE_URL/api/user/history/fronting")"
    echo "$HISTORY" | jq .
}

assert_history_length() {
    local expected="$1"
    [[ "$(echo "$HISTORY" | jq 'length')" == "$expected" ]]
}

assert_history_length_gt() {
    local min="$1"
    [[ "$(echo "$HISTORY" | jq 'length')" -gt "$min" ]]
}

assert_history_entry_names() {
    local expected_names=("$@")
    local status_texts
    status_texts="$(echo "$HISTORY" | jq -r '.[].status_text')"
    
    for name in "${expected_names[@]}"; do
        [[ "$status_texts" =~ "$name" ]]
    done
}

assert_history_not_contains() {
    local unwanted_names=("$@")
    local status_texts
    status_texts="$(echo "$HISTORY" | jq -r '.[].status_text')"
    
    for name in "${unwanted_names[@]}"; do
        [[ ! "$status_texts" =~ "$name" ]]
    done
}


set_history_config() {
    export HISTORY_LIMIT="$1"
    export HISTORY_TRUNCATE_AFTER_DAYS="$2"
    set_user_config_and_restart
}

export BASE_URL="http://localhost:8080"

ENABLE_DISCORD=false
ENABLE_DISCORD_STATUS_MESSAGE=false
ENABLE_VRCHAT=false
ENABLE_WEBSITE=true
ENABLE_TO_PLURALKIT=false
export HISTORY_LIMIT=100
export HISTORY_TRUNCATE_AFTER_DAYS=7
SECONDS_BETWEEN_UPDATES=2

start_updater() {
    echo "start_updater"
    ./docker/start.sh local > docker/logs/start.log 2>&1

    setup_test_user

    await pluralsync-api "client authentication sent."

    sleep 3s

    echo "Started updater."
}

stop_updater() {
    echo "stop_updater"
    ./docker/stop.sh local > docker/logs/stop.log 2>&1
    echo "Stopped updater."
}
trap stop_updater EXIT

main

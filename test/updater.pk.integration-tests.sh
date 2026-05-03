#!/bin/bash

set -euo pipefail

[[ "$SPS_API_TOKEN" != "" ]]

[[ "$SPS_API_WRITE_TOKEN" != "" ]]

[[ "$DISCORD_STATUS_MESSAGE_TOKEN" != "" ]]

export DISCORD_STATUS_MESSAGE_UPDATER_AVAILABLE=true
ENABLE_DISCORD_STATUS_MESSAGE=true
ENABLE_VRCHAT=false
ENABLE_DISCORD=false
ENABLE_WEBSITE=false
ENABLE_TO_PLURALKIT=false
export ENABLE_FROM_PLURALKIT=true
export ENABLE_FROM_SP=false

source ./test/source.sh
source ./test/plural_system_to_test.sh

main() {
    stop_updater
    ./steps/12-backend-cargo-build.sh

    start_updater


    set_system_fronts_set "A"
    sleep "$SECONDS_BETWEEN_UPDATES"s
    check_system_fronts_set "A"

    set_system_fronts_set "B"
    sleep "$SECONDS_BETWEEN_UPDATES"s
    check_system_fronts_set "B"

    set_system_fronts_set "C"
    sleep "$SECONDS_BETWEEN_UPDATES"s
    check_system_fronts_set "C-limited-visibility"

    set_system_fronts_set "D"
    sleep "$SECONDS_BETWEEN_UPDATES"s
    check_system_fronts_set "D-limited-visibility"


    clear_all_fronts
    echo "✅✅✅ Updater Integration Test ✅✅✅"
}


check_system_fronts_set() {
    SET="$1"
    echo "check_system_fronts_set '$SET'"

    if [[ "$SET" == "A" ]]; then
        check_discord_status_string_equals "F: Annalea 💖 A., Borgnen 👍 B., Daenssa 📶 D."
    elif [[ "$SET" == "B" ]]; then
        check_discord_status_string_equals "F: tešt ▶️ t. 漢字 クケ 汉字 漢字"
    elif [[ "$SET" == "C-limited-visibility" ]]; then
        check_discord_status_string_equals "F: NK notif-ok"
    elif [[ "$SET" == "D-limited-visibility" ]]; then
        check_discord_status_string_equals "F: pbucket-member-yes"
    else
        return 1
    fi
}


check_discord_status_string_equals() {
    EXPECTED="$1"

    RESPONSE="$(curl -s \
        "https://discord.com/api/v10/users/@me/settings" \
        -H "Authorization: $DISCORD_STATUS_MESSAGE_TOKEN"
    )"

    STATUS="$( echo "$RESPONSE" | jq -r .custom_status.text )"

    echo "Discord Status Check: '$STATUS' =? '$EXPECTED'"

    [[ "$STATUS" == "$EXPECTED" ]]
}

check_to_pluralkit_fronters_equals() {
    EXPECTED="$1"
    
    RESPONSE="$(
        curl -s \
            -H "Content-Type: application/json" -H "Authorization: $PLURALKIT_TOKEN" \
            "https://api.pluralkit.me/v2/systems/@me/switches?limit=1"
    )"
    
    MEMBERS="$( echo "$RESPONSE" | jq -r '.[0].members | sort | join(",")' )"

    echo "PluralKit Fronters Check: '$MEMBERS' =? '$EXPECTED'"

    [[ "$MEMBERS" == "$EXPECTED" ]]
}


export BASE_URL="http://localhost:8080"

start_updater() {
    echo "start_updater"
    ./docker/start.sh local > docker/logs/start.log 2>&1

    setup_test_user

    await pluralsync-api "update statues | ok"

    echo "Started Updater."
}

stop_updater() {
    echo "stop_updater"
    ./docker/stop.sh local > docker/logs/stop.log 2>&1
    echo "Stopped Updater."
}
trap stop_updater EXIT

main

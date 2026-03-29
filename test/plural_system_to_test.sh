#!/bin/bash

export ANNALEA_ID="683f23e79aa188caf3000000"
export ANNALEA_ID_PK="wgpkrn"
export BORGNEN_ID="683f23f49aa189caf3000000"
export BORGNEN_ID_PK="bxsrbg"
export CLENNTRO_ID="683f24009aa18acaf3000000"
export CLENNTRO_ID_PK="xtythx"
export DAENSSA_ID="683f24179aa18bcaf3000000"
export DAENSSA_ID_PK="crocku"
export TEST_MEMBER_ID="683f243e9aa18ccaf3000000"
export TEST_MEMBER_ID_PK="zphjou"
export NOTIF_OK="68e1950560bb6cfa4a000000"
export NOTIF_OK_PK="rovknb"
export NOTIF_NOT_OK="68e1952060bb6dfa4a000000"
export NOTIF_NOT_OK_PK="uncjco"
export ARCHIVED_NOTIF_OK="68e195b960bb70fa4a000000"
export ARCHIVED_NOTIF_OK_PK="tabobe"
export PBUCKET_MEMBER_NO="68e23ebed3877fbeb6000000"
export PBUCKET_MEMBER_NO_PK="nguonb"
export PBUCKET_MEMBER_YES="68e23eb0d3877ebeb6000000"
export PBUCKET_MEMBER_YES_PK="kapnfj"
export CUSTOM_FRONT_1_ID="688d41c8aa2e477e53000000"


set_system_fronts_set() {
    SET="$1"

    clear_all_fronts

    if [[ "$SET" == "A" ]]; then
        set_to_front_sp "$ANNALEA_ID"
        set_to_front_sp "$BORGNEN_ID" "$BORGNEN_ID_PK"
        set_to_front_sp "$DAENSSA_ID" "$DAENSSA_ID_PK"
        set_to_front_sp "$CUSTOM_FRONT_1_ID"

        set_to_front_pk "[\"$ANNALEA_ID_PK\",\"$BORGNEN_ID_PK\",\"$DAENSSA_ID_PK\"]"
    elif [[ "$SET" == "B" ]]; then
        set_to_front_sp "$TEST_MEMBER_ID"
        set_to_front_pk "[\"$TEST_MEMBER_ID_PK\"]"
    elif [[ "$SET" == "C" ]]; then
        set_to_front_sp "$NOTIF_OK"
        set_to_front_sp "$NOTIF_NOT_OK"
        set_to_front_sp "$ARCHIVED_NOTIF_OK"
        set_to_front_pk "[\"$NOTIF_OK_PK\",\"$NOTIF_NOT_OK_PK\",\"$ARCHIVED_NOTIF_OK_PK\"]"
    elif [[ "$SET" == "D" ]]; then
        set_to_front_sp "$PBUCKET_MEMBER_NO"
        set_to_front_sp "$PBUCKET_MEMBER_YES" "$PBUCKET_MEMBER_NO_PK"
        set_to_front_pk  "[\"$PBUCKET_MEMBER_NO_PK\",\"$PBUCKET_MEMBER_YES_PK\"]"
    else
        return 1
    fi

    push_pk_mock_webhook_dispatch_after_delay
}


set_to_front_sp() {
    if [[ "$ENABLE_FROM_SP" != "true" ]]; then return 0 ; fi
    FRONTER_ID="$1"
    FRONT_ID="$(openssl rand -hex 12)" # produces valid 24 hexdec digits
    UNIX_MILLIS_CURRENT="$(date +%s%3N)"
    UNIX_MILLIS_5_MIN_AGO="$((UNIX_MILLIS_CURRENT - 5*60*1000))"
    curl --silent --fail-with-body -L "https://api.apparyllis.com/v1/frontHistory/$FRONT_ID" \
        -H 'Content-Type: application/json' \
        -H "Authorization: $SPS_API_WRITE_TOKEN" \
        -d "{
            \"customStatus\": \"\",
            \"custom\": false,
            \"live\": true,
            \"startTime\": $UNIX_MILLIS_5_MIN_AGO,
            \"member\": \"$FRONTER_ID\"
        }" > /dev/null
    echo "Set member/custom-front $FRONTER_ID to front (id: $FRONT_ID)."
    rate_limiting_delay
}

push_pk_mock_webhook_dispatch_after_delay() {
    if [[ "$ENABLE_FROM_PLURALKIT" == "true" ]]; then
        sleep 1.5s
        echo "Sending mocked pluralkit dispatch for $USER_ID"
        curl -s -H "Content-Type: application/json" -X POST "http://localhost:8080/api/webhook/pluralkit/$USER_ID" -d "{\"type\": \"CREATE_SWITCH\", \"signing_token\": \"$PK_WEBHOOK_SIGNING_TOKEN\"}"
    fi
}

set_to_front_pk() {
    if [[ "$ENABLE_FROM_PLURALKIT" != "true" ]]; then return 0 ; fi
    echo "Setting pk switch: $1"
    curl -s --fail-with-body -o /dev/null \
        -H "Authorization: $PLURALKIT_TOKEN" -H "Content-Type: application/json" \
        -X POST "https://api.pluralkit.me/v2/systems/@me/switches" \
        -d "{\"members\":$1}"
}


clear_all_fronts() {
    echo "Clearing all active fronts."

    if [[ "$ENABLE_FROM_SP" == "true" ]]; then 
        FRONTER_IDS="$(
            curl --silent \
                -L 'https://api.apparyllis.com/v1/fronters/' \
                -H "Authorization: $SPS_API_WRITE_TOKEN" |
                jq -r '.[].id'
        )"

        if [[ "$FRONTER_IDS" == "" ]]; then
            return 0
        fi

        while read fronter_id; do
            
            echo "Clearing front (id=$fronter_id)"
            
            curl --silent -L -X PATCH "https://api.apparyllis.com/v1/frontHistory/$fronter_id" \
                -H 'Content-Type: application/json' \
                -H "Authorization: $SPS_API_WRITE_TOKEN" \
                -d '{
                    "live": false,
                    "startTime": 0,
                    "endTime": 15,
                    "customStatus": "",
                    "custom": false
                }'
            
            rate_limiting_delay

        done <<< "$FRONTER_IDS"
    else
        curl -s --fail-with-body -o /dev/null \
            -H "Authorization: $PLURALKIT_TOKEN" -H "Content-Type: application/json" \
            -X POST "https://api.pluralkit.me/v2/systems/@me/switches" \
            -d '{"members":[]}' || true
        # pk returns 400 if the switch is already empty
    fi
    echo "Done."
}

rate_limiting_delay() {
    sleep 0.3s
}
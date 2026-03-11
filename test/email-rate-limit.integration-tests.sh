#!/bin/bash

set -euo pipefail

# Set a very low rate limit for testing (5 emails per day)
export SMTP_EMAIL_RATE_LIMIT_PER_DAY=5
# Enable dev mode to see email content in logs instead of actually sending
export DANGEROUS_LOCAL_DEV_MODE_PRINT_TOKENS_INSTEAD_OF_SEND_EMAIL=true

export DISCORD_STATUS_MESSAGE_UPDATER_AVAILABLE=false
ENABLE_DISCORD_STATUS_MESSAGE=false
ENABLE_VRCHAT=false
ENABLE_DISCORD=false
ENABLE_WEBSITE=false
ENABLE_TO_PLURALKIT=false

source ./test/source.sh

main() {
    stop_backend
    ./steps/12-backend-cargo-build.sh

    echo "Rate limit configured to $SMTP_EMAIL_RATE_LIMIT_PER_DAY emails per day"

    start_backend

    # Note: Email slots are consumed by:
    # 1. setup_test_user verification email (test@example.com)
    # 2. user1@example.com verification email
    # 3. Announcement email to test@example.com (sent automatically on startup)
    # 4. user2@example.com verification email
    # 5. user3@example.com verification email
    # Total: 5 emails (limit reached)
    # user4 will fail because the rate limit is exhausted

    # Test 1: Register users until rate limit is reached
    echo ""
    echo "=== Test 1: Register users until rate limit ==="
    register_user "user1@example.com" "Password1!"
    register_user "user2@example.com" "Password2!"
    register_user "user3@example.com" "Password3!"

    echo ""
    echo "Successfully registered 3 users (rate limit reached: 1 setup + 1 announcement + 3 users = 5)"

    # Test 2: Try to register one more user - should fail due to rate limit
    echo ""
    echo "=== Test 2: Attempt to exceed rate limit ==="
    if register_user_succeeded "user4@example.com" "Password4!"; then
        echo "❌ FAILED: Expected registration to fail due to rate limit, but it succeeded"
        exit 1
    else
        echo "✅ Registration correctly failed due to rate limit"
    fi

    # Test 3: Verify rate limit is hit in logs (EMAIL_RATE_LIMIT_THRESHOLD_EXCEEDED means limit reached)
    echo ""
    echo "=== Test 3: Verify rate limit hit in logs ==="
    if check_logs_contain "EMAIL_RATE_LIMIT_THRESHOLD_EXCEEDED: 5 emails sent today, threshold is 5"; then
        echo "✅ Rate limit hit found in logs (5 emails sent, threshold 5)"
    else
        echo "❌ FAILED: Rate limit hit not found in logs"
        docker logs pluralsync-api 2>&1 | tail -50
        exit 1
    fi

    # Test 4: Count emails sent in logs
    echo ""
    echo "=== Test 4: Count emails sent ==="
    EMAIL_COUNT="$(count_dev_mode_emails)"
    echo "Emails sent (from logs): $EMAIL_COUNT"
    if [[ "$EMAIL_COUNT" -eq 5 ]]; then
        echo "✅ Exactly 5 emails sent as expected (1 setup + 1 announcement + 3 users)"
    else
        echo "❌ FAILED: Expected 5 emails, got $EMAIL_COUNT"
        exit 1
    fi

    # Test 5: Verify no additional emails were sent after limit
    echo ""
    echo "=== Test 5: Verify no additional emails after limit ==="
    FINAL_COUNT="$(count_dev_mode_emails)"
    if [[ "$FINAL_COUNT" -eq 5 ]]; then
        echo "✅ No additional emails sent after limit (still at 5)"
    else
        echo "❌ FAILED: Expected 5 emails, got $FINAL_COUNT"
        exit 1
    fi

    echo ""
    echo "✅✅✅ All Email Rate Limit Tests Passed ✅✅✅"
}

register_user() {
    EMAIL="$1"
    PASSWORD="$2"
    echo "Registering $EMAIL ..."

    JSON="{
        \"email\": { \"inner\": \"$EMAIL\" },
        \"password\": { \"inner\": { \"inner\": \"$PASSWORD\" } }
    }"

    curl -s --fail-with-body \
        -H "Content-Type: application/json" \
        -d "$JSON" \
        "$BASE_URL/api/user/register" > /dev/null

    echo "✅ Registered $EMAIL"
}

register_user_succeeded() {
    EMAIL="$1"
    PASSWORD="$2"
    echo "Attempting to register $EMAIL (should fail) ..."

    JSON="{
        \"email\": { \"inner\": \"$EMAIL\" },
        \"password\": { \"inner\": { \"inner\": \"$PASSWORD\" } }
    }"

    set +e
    curl -s --fail-with-body \
        -H "Content-Type: application/json" \
        -d "$JSON" \
        "$BASE_URL/api/user/register" > /dev/null 2>&1
    EXIT_CODE=$?
    set -e

    if [[ $EXIT_CODE -eq 0 ]]; then
        echo "Registration unexpectedly succeeded"
        return 0
    fi

    echo "Registration failed as expected"
    return 1
}

count_dev_mode_emails() {
    docker logs pluralsync-api 2>&1 | grep -c "\[DEV MODE - EMAIL NOT SENT\]" || echo "0"
}

check_logs_contain() {
    echo "Checking logs for: '$1'"
    docker logs pluralsync-api 2>&1 | grep -q "$1"
}

export BASE_URL="http://localhost:8080"

start_backend() {
    echo "start_backend"
    ./docker/start.sh local > docker/logs/start.log 2>&1

    setup_test_user

    await pluralsync-api "Waiting for next update trigger..."

    echo "Started backend."
}

stop_backend() {
    echo "stop_backend"
    ./docker/stop.sh local > docker/logs/stop.log 2>&1
    echo "Stopped backend."
}
trap stop_backend EXIT

main

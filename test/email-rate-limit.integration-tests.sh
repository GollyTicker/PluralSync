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
    # 1. setup_test_user verification email
    # 2. User registration verification emails
    # Announcement emails are NOT sent because all users are created AFTER the backend starts
    # (announcements are only sent to users registered before the announcement was created)
    #
    # With rate limit of 5: 1 (setup) + NUM_USERS = 5
    # Therefore: NUM_USERS = SMTP_EMAIL_RATE_LIMIT_PER_DAY - 1 = 4 users
    NUM_USERS=$((SMTP_EMAIL_RATE_LIMIT_PER_DAY - 1))
    echo "Will register $NUM_USERS users before rate limit is reached"

    # Test 1: Register users until rate limit is reached
    echo ""
    echo "=== Test 1: Register users until rate limit ==="
    for i in $(seq 1 $NUM_USERS); do
        register_user "user${i}@example.com" "Password${i}!"
    done

    EXPECTED_EMAIL_COUNT=$((1 + NUM_USERS))
    echo ""
    echo "Successfully registered $NUM_USERS users (rate limit reached: 1 setup verification + $NUM_USERS user verifications = $EXPECTED_EMAIL_COUNT)"

    # Test 2: Try to register one more user - email sending should fail due to rate limit
    # Note: User registration itself succeeds, but the verification email won't be sent
    echo ""
    echo "=== Test 2: Attempt to exceed rate limit ==="
    NEXT_USER=$((NUM_USERS + 1))
    echo "Attempting to register user${NEXT_USER}@example.com (email sending should fail) ..."
    
    # Register the user (this will succeed, but email should fail)
    register_user "user${NEXT_USER}@example.com" "Password${NEXT_USER}!" || true
    
    # Verify rate limit error was logged for this user's email
    if check_logs_contain "Email rate limit exceeded.*user${NEXT_USER}@example.com"; then
        echo "✅ Email sending correctly failed due to rate limit for user${NEXT_USER}"
    else
        echo "❌ FAILED: Expected rate limit error for user${NEXT_USER} email, but not found"
        docker logs pluralsync-api 2>&1 | tail -50
        exit 1
    fi

    # Test 3: Verify rate limit is hit in logs (EMAIL_RATE_LIMIT_THRESHOLD_EXCEEDED means limit reached)
    echo ""
    echo "=== Test 3: Verify rate limit hit in logs ==="
    if check_logs_contain "EMAIL_RATE_LIMIT_THRESHOLD_EXCEEDED: $EXPECTED_EMAIL_COUNT emails sent today, threshold is $EXPECTED_EMAIL_COUNT"; then
        echo "✅ Rate limit hit found in logs ($EXPECTED_EMAIL_COUNT emails sent, threshold $EXPECTED_EMAIL_COUNT)"
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
    if [[ "$EMAIL_COUNT" -eq "$EXPECTED_EMAIL_COUNT" ]]; then
        echo "✅ Exactly $EXPECTED_EMAIL_COUNT emails sent as expected (1 setup verification + $NUM_USERS user verifications)"
    else
        echo "❌ FAILED: Expected $EXPECTED_EMAIL_COUNT emails, got $EMAIL_COUNT"
        exit 1
    fi

    # Test 5: Verify no additional emails were sent after limit
    echo ""
    echo "=== Test 5: Verify no additional emails after limit ==="
    FINAL_COUNT="$(count_dev_mode_emails)"
    if [[ "$FINAL_COUNT" -eq "$EXPECTED_EMAIL_COUNT" ]]; then
        echo "✅ No additional emails sent after limit (still at $EXPECTED_EMAIL_COUNT)"
    else
        echo "❌ FAILED: Expected $EXPECTED_EMAIL_COUNT emails, got $FINAL_COUNT"
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

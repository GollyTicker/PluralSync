# PluralKit Webhook Integration - Implementation Summary

## Overview
This document summarizes the basic PluralKit webhook integration implementation for PluralSync.

## Files Created/Modified

### 1. Database Migration
**File:** `docker/migrations/019_add_pluralkit_webhooks.sql`

Adds the following columns to the `users` table:
- `enable_from_pluralkit` - Boolean flag to enable/disable webhook processing
- `pluralkit_webhook_signing_token_hash` - Hashed signing token for signature verification
- `pluralkit_webhook_signing_token_salt` - Per-user salt for HMAC signing
- `pluralkit_webhook_last_ping_received_at` - Timestamp of last health check ping

### 2. PluralKit Webhook Types
**File:** `base-src/src/platforms/pluralkit.rs`

Defines the webhook payload structures:
- `PluralKitWebhookEvent` - Enum with all PK event types:
  - `CreateSwitch`, `UpdateSwitch`, `DeleteSwitch`, `DeleteAllSwitches` (trigger fetches)
  - `CreateMember`, `UpdateMember`, `DeleteMember` (ignored)
  - `Ping` (health check)
- `PluralKitWebhookPayload` - Main webhook payload structure
- `PluralKitSystem`, `PluralKitSwitch`, `PluralKitMember` - Supporting structures

**Key Methods:**
- `should_trigger_fetch()` - Returns true for switch-related events
- `is_ping()` - Returns true for ping events

**Tests:** 4 unit tests for event filtering and payload deserialization

### 3. Webhook Handler
**File:** `src/plurality/pluralkit_webhook.rs`

Implements the webhook endpoint: `POST /webhook/pluralkit/<user_id>`

**Features:**
- Parses incoming webhook payloads
- Validates user ID format (UUID)
- Handles PING events (returns "pong")
- Filters events by type (switch vs non-switch)
- Logs all events with appropriate severity levels
- Returns appropriate HTTP status codes:
  - `200 OK` with "pong" for PING events
  - `200 OK` with "ignored" for non-switch events
  - `200 OK` with "updated" for switch events (when fetch succeeds)
  - `400 Bad Request` for invalid payloads or user IDs
  - `500 Internal Server Error` for fetch failures

**Metrics Tracked:**
- `pluralkit_webhook_requests_total` - Total webhook requests received
- `pluralkit_webhook_validation_failures_total` - Signature validation failures
- `pluralkit_webhook_relevant_events_total` - Switch events that trigger fetches
- `pluralkit_webhook_ping_events_total` - Health check pings received
- `pluralkit_webhook_fetch_triggered_total` - Successful fetch triggers

**Tests:** 5 unit tests for payload parsing, event filtering, and user ID validation

### 4. Fronter Struct Update
**File:** `src/plurality/simply_plural_model.rs`

Added `pronouns: Option<String>` field to the `Fronter` struct to support pronouns from PluralKit members.

**Updated:**
- `From<Member>` implementation
- `From<CustomFront>` implementation
- All test fixtures

### 5. Integration Points

**File:** `src/main.rs`
- Added webhook route to Rocket server

**File:** `src/metrics.rs`
- Registered all PluralKit webhook metrics

**File:** `Cargo.toml`
- Added `hmac` dependency for signature verification
- Added `serde_json` to base-src

### 6. Test Suite
**File:** `test/pluralkit-webhook.integration-tests.sh`

Comprehensive integration test script that verifies:
1. PING event handling
2. CREATE_SWITCH event handling
3. UPDATE_SWITCH event handling
4. DELETE_SWITCH event handling
5. DELETE_ALL_SWITCHES event handling
6. Non-switch event filtering (CREATE_MEMBER, UPDATE_MEMBER, DELETE_MEMBER)
7. Invalid payload handling
8. Invalid user ID handling
9. Metrics exposure verification

**Usage:**
```bash
export PLURALKIT_TOKEN='your_token_here'
./test/pluralkit-webhook.integration-tests.sh
```

## Current Limitations (TODO)

### 1. Signature Verification
The webhook signature verification is not yet implemented. The webhook currently accepts all requests without verifying the HMAC-SHA256 signature.

**Required for production:**
- Store raw webhook secret (encrypted) in database
- Implement HMAC-SHA256 verification
- Add signature header extraction from HTTP request

### 2. Full Fetch Logic
The `fetch_and_update_fronters` function is a stub that only logs events.

**Required for full functionality:**
- Fetch user config with PluralKit token from database
- Call PluralKit API to get latest switch
- Fetch member details from PluralKit API
- Convert to `Fronter` structs
- Send to fronter channel to trigger updaters

### 3. Webhook Registration API
No API endpoints exist for users to register webhooks with PluralKit.

**Required:**
- `POST /api/user/config/pluralkit/webhook/register` - Generate secret and return webhook URL
- `DELETE /api/user/config/pluralkit/webhook` - Unregister webhook
- `GET /api/user/config/pluralkit/webhook/status` - Get webhook status

### 4. Frontend Configuration UI
No UI components for users to configure PluralKit webhooks.

**Required:**
- `PluralKitWebhookConfig.vue` component
- Enable/disable toggle
- Display webhook URL with copy button
- Show last ping received timestamp

## Testing Results

### Unit Tests
All 62 Rust unit tests pass, including:
- 4 tests in `base-src/src/platforms/pluralkit.rs`
- 5 tests in `src/plurality/pluralkit_webhook.rs`

### Build Verification
- ✅ Backend builds successfully (`./steps/12-backend-cargo-build.sh`)
- ✅ Linting passes (`./steps/10-lint.sh`)
- ✅ All Rust tests pass (`./test/rust-tests.sh`)

## API Reference

### Webhook Endpoint

**URL:** `POST /webhook/pluralkit/{user_id}`

**Request Body:**
```json
{
  "event": "CREATE_SWITCH",
  "system": {
    "id": "system_id",
    "uid": "system_uid"
  },
  "switch": {
    "id": "switch_id",
    "members": ["member1", "member2"],
    "timestamp": "2024-01-01T12:00:00Z"
  },
  "timestamp": "2024-01-01T12:00:00Z"
}
```

**Optional Headers:**
- `X-Signature` - HMAC-SHA256 signature (TODO: implement verification)

**Responses:**
- `200 OK` with body "pong" - PING event
- `200 OK` with body "ignored" - Non-switch event
- `200 OK` with body "updated" - Switch event processed successfully
- `400 Bad Request` - Invalid payload or user ID
- `500 Internal Server Error` - Processing error

## Next Steps

1. **Implement signature verification** - Critical for security
2. **Implement full fetch logic** - Core functionality
3. **Add webhook registration API** - User configuration
4. **Create frontend UI** - User experience
5. **Add integration tests** - End-to-end verification with real PluralKit system
6. **Add metrics dashboard** - Monitoring and alerting
7. **Documentation** - User guide for webhook setup

## Security Considerations

- Webhook URLs contain user ID, which acts as a capability token
- HTTPS should be enforced in production
- Signature verification is critical to prevent spoofing
- Rate limiting should be considered for webhook endpoints
- Logging should redact sensitive information

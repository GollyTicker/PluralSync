# TODO: PluralKit Webhook Forwarding

## Problem
PluralKit allows only **one webhook per system**. Users cannot send PluralKit events to multiple destinations simultaneously.

## Solution
PluralSync forwards incoming PluralKit webhook events to user-configured forwarding URLs **before** processing them locally.

## Design Decisions

### Forwarding Behavior
- **Fire-and-forget**: No retries on failure
- **Async**: Spawned as independent tokio tasks
- **Forward BEFORE local processing**: Raw body forwarded immediately, then parsed/validated locally
- **Resilient to JSON parse failure**: Forwarding happens regardless of whether our own JSON parsing succeeds
- **Re-use PluralKit signing token**: Forward the original payload unchanged

### Failure Handling
- **PING events only** track consecutive failures for disabling
- **Normal events**: Errors logged, ignored for failure counting
- **3 consecutive PING failures** → disable forward + notify user via email
- **Daily cron job** (24h intervals) sends PING tests to all enabled forwards

### Initial Setup Flow
1. User adds a new forwarding URL
2. Backend sends **two** test webhooks to the URL:
   - Valid signing token → expects `200 OK`
   - Invalid signing token → expects `401 Unauthorized`
3. If both responses match expectations → URL registered in database
4. If not → reject the URL with error message

### No Rate Limiting on Forwards
### No Custom Authentication on Forwards

## Database

### New Table: `pluralkit_webhook_forwards`
| Column | Type | Notes |
|--------|------|-------|
| `id` | UUID PRIMARY KEY | Auto-generated |
| `forward_url` | TEXT | Destination URL |
| `enabled` | BOOLEAN | enabled-or-not |
| `last_error` | TEXT NULLABLE | Error from last failed PING event |
| `consecutive_failures` | INTEGER | Default `0`, reset on PING success |
| `last_test_sent_at` | TIMESTAMPTZ | Last PING test sent |

### Migration: `025_pluralkit_webhook_forwards.sql`

## Implementation Tasks

### 1. Database Migration
- [ ] Create `025_pluralkit_webhook_forwards.sql`
- [ ] Add indexes on `user_id` and `enabled`

### 2. Database Queries (`src/database/pluralkit_webhook_forwards_queries.rs`)
- [ ] `get_enabled_forwards(user_id)` → `Vec<WebhookForward>`
- [ ] `add_forward(user_id, url)` → `WebhookForward`
- [ ] `update_forward_status(id, success, error)` → `()`
- [ ] `delete_forward(id)` → `()`
- [ ] `get_all_enabled_forwards()` → `Vec<WebhookForward>` (for cron job)
- [ ] `disable_forward(id)` → `()`
- [ ] `reset_consecutive_failures(id)` → `()`
- [ ] `increment_consecutive_failures(id, error)` → `()`
- [ ] `update_last_test_sent_at(id)` → `()`

### 3. Data Structures (`src/database/pluralkit_webhook_forwards.rs` or inline)
- [ ] `struct PluralKitWebhookForward { id, user_id, forward_url, enabled, last_error, consecutive_failures, last_test_sent_at }`

### 4. Webhook Forwarder Module (`src/plurality/pluralkit_webhook_forwarder.rs`)
- [ ] `async fn forward_to_url(client, url, payload_body, signing_token, user_id, forward_id)` → result
- [ ] `async fn test_forward_with_valid_token(client, url, signing_token)` → expects 200
- [ ] `async fn test_forward_with_invalid_token(client, url, signing_token)` → expects 401
- [ ] `async fn send_ping_test(client, forward)` → update db based on result
- [ ] Helper: update consecutive failures / disable logic

### 5. Refactor `pluralkit_webhook_api.rs`
- [ ] **FIRST**: Spawn async forward tasks to all enabled URLs (fire-and-forget, raw body)
- [ ] **THEN**: Parse JSON, validate token, process locally (existing logic)
- [ ] Ensure forwarding is independent of local processing success/failure

### 6. API Endpoints (`src/plurality/pluralkit_webhook_forwards_api.rs`)
- [ ] `POST /api/user/pluralkit/forwards` — Add new forward (includes setup test)
- [ ] `GET /api/user/pluralkit/forwards` — List all forwards for user
- [ ] `DELETE /api/user/pluralkit/forwards/<forward_id>` — Remove a forward
- [ ] `POST /api/user/pluralkit/forwards/<forward_id>/test` — Manual PING test
- [ ] `PUT /api/user/pluralkit/forwards/<forward_id>/enable` — Re-enable disabled forward
- [ ] Register routes in `main.rs`

### 7. Daily Cron Job (`src/plurality/pluralkit_webhook_forwards_cron.rs`)
- [ ] Query all enabled forwards
- [ ] Send PING to each
- [ ] Update `consecutive_failures`, `last_error`, `last_test_sent_at`
- [ ] Disable + email notify if `consecutive_failures >= 3`
- [ ] Register cron job in `main.rs` at `DAILY_AT_0400`

### 8. Email Notification (`src/users/email.rs`)
- [ ] Add `EmailType::PluralKitWebhookForwardDeactivated`
- [ ] `send_pluralkit_webhook_forward_deactivated_email(user_email, forward_url, last_error)`
- [ ] Email explains: URL disabled after 3 failed PINGs, how to re-enable/remove

### 9. Prometheus Metrics
- [ ] `PLURALKIT_WEBHOOK_FORWARDS_TOTAL` — counter, labels: `[user_id, forward_id, status]`
- [ ] `PLURALKIT_WEBHOOK_FORWARD_FAILURES_TOTAL` — counter, labels: `[user_id, forward_id]`
- [ ] `PLURALKIT_WEBHOOK_FORWARDS_DISABLED_TOTAL` — counter, labels: `[user_id]`
- [ ] `PLURALKIT_WEBHOOK_FORWARD_PING_TOTAL` — counter, labels: `[user_id, forward_id, success]`

### 10. Frontend (`frontend/src/components/PluralKitConfigPanel.vue`)
- [ ] Section: "Webhook Forwarding URLs"
- [ ] Input field for URL + "Add & Test" button
- [ ] List existing forwards showing: URL, enabled status, last error, consecutive failures
- [ ] Delete button per forward
- [ ] Re-enable button for disabled forwards
- [ ] Manual "Test" button per forward
- [ ] Loading/error states during setup test
- [ ] TypeScript types for forward struct (generate via specta bindings)

### 11. specta TypeScript Bindings
- [ ] Export `PluralKitWebhookForward` struct for frontend

### 12. Testing
- [ ] Unit tests for forwarder logic
- [ ] Unit tests for failure counting / disable logic
- [ ] Integration tests for API endpoints
- [ ] Integration tests for cron job behavior

## Key Files to Create/Modify

| File | Action |
|------|--------|
| `docker/migrations/025_pluralkit_webhook_forwards.sql` | **CREATE** |
| `src/database/pluralkit_webhook_forwards_queries.rs` | **CREATE** |
| `src/plurality/pluralkit_webhook_forwarder.rs` | **CREATE** |
| `src/plurality/pluralkit_webhook_forwards_api.rs` | **CREATE** |
| `src/plurality/pluralkit_webhook_forwards_cron.rs` | **CREATE** |
| `src/plurality/pluralkit_webhook_api.rs` | **MODIFY** (forward before process) |
| `src/plurality/mod.rs` | **MODIFY** (add new modules) |
| `src/users/email.rs` | **MODIFY** (add email type + function) |
| `src/main.rs` | **MODIFY** (register routes + cron) |
| `src/lib.rs` | Possibly modify (exports) |
| `frontend/src/components/PluralKitConfigPanel.vue` | **MODIFY** (add forwarding UI) |
| `frontend/src/pluralsync.bindings.ts` | **AUTO-GENERATED** (re-run bindings generator) |

# PluralKit Webhooks Specification

## Goal

Add real-time PluralKit switch change notifications to PluralSync, enabling instant sync when users update their fronters in PluralKit.

---

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| **Webhooks are notifications only** | On relevant events, fetch entire system fresh from PK API — simpler than parsing payloads |
| **One endpoint per user** | URL pattern: `/webhook/pluralkit/{system_id}` — system_id routes to user directly |
| **No polling fallback** | Ignore temporary failures; webhooks are best-effort |
| **One PK system per user** | User's configured PK token determines their system |
| **Independent from SP WebSocket** | Both can coexist; fetches happen asynchronously |
| **Fetch from primary source only** | Enrichment from secondary source per existing `FieldSourcingConfig` |

---

## Architecture

```
PluralKit → POST /webhook/pluralkit/{system_id}
              ↓
          Validate signing_token
              ↓
          Check event type (CREATE_SWITCH, UPDATE_SWITCH, etc.)
              ↓
          Trigger full system fetch from PK API
              ↓
          Send to fronter channel → Updaters (VRChat, Discord, etc.)
```

---

## Relevant Event Types

Only these events trigger a full fetch:
- `CREATE_SWITCH`
- `UPDATE_SWITCH`
- `DELETE_SWITCH`
- `DELETE_ALL_SWITCHES`

`PING` events update health monitoring but don't trigger fetches. All other events are ignored.

---

## Database Schema

```sql
ALTER TABLE user_config ADD COLUMN enable_from_pluralkit BOOLEAN DEFAULT FALSE;
ALTER TABLE user_config ADD COLUMN pluralkit_webhook_signing_token_hash BYTEA DEFAULT NULL;
ALTER TABLE user_config ADD COLUMN pluralkit_webhook_signing_token_salt BYTEA DEFAULT NULL;
ALTER TABLE user_config ADD COLUMN pluralkit_webhook_last_ping_received_at TIMESTAMPTZ DEFAULT NULL;

CREATE INDEX idx_user_config_pluralkit_system_id ON user_config(pluralkit_system_id);
```

---

## Implementation Components

### 1. Webhook Endpoint (`POST /webhook/pluralkit/{system_id}`)

**Responsibilities:**
- Look up user by `system_id`
- Validate `signing_token` (constant-time comparison against hashed stored token)
- Filter by event type
- Trigger full system fetch on relevant events
- Return `200 OK` (or `401` on invalid token, `404` on unknown system)

### 2. Full System Fetch

**API calls (per webhook trigger):**
1. `GET /systems/{system_id}/switches/latest` — get current switch with member IDs
2. `GET /systems/{system_id}/members` — fetch all members in one call
3. Match switch member IDs to member objects

**Not individual member fetches** — use bulk member endpoint for efficiency.

### 3. Webhook Registration

**Endpoint:** `POST /api/user/config/pluralkit/webhook/register`

**Process:**
1. Generate random signing token (256-bit)
2. Hash with salt, store in DB
3. Call PK API: `PUT /systems/{system_id}/webhook` with URL `https://pluralsync.example.com/webhook/pluralkit/{system_id}`
4. Return webhook URL to user

**User configures in PluralKit:** `pk;s webhook <returned_url>`

### 4. Frontend Configuration

**Component:** `PluralKitWebhookConfig.vue`

**Features:**
- Enable/disable toggle
- Display webhook URL with copy button
- Show last PING received timestamp
- Instructions: `pk;s webhook <url>`

---

## Metrics

- `pluralkit_webhook_requests_total` — all incoming requests
- `pluralkit_webhook_validation_failures_total` — invalid tokens
- `pluralkit_webhook_relevant_events_total` — events that triggered fetches
- `pluralkit_webhook_ping_events_total` — PING events received
- `pluralkit_webhook_fetch_triggered_total` — full fetches triggered
- `pluralkit_webhook_time_since_last_ping_ms` — health gauge

---

## File Structure

```
src/
  plurality/
    pluralkit_webhook.rs          # HTTP handler
    pluralkit_webhook_manager.rs  # Registration/unregistration
    sources/
      pluralkit_source.rs         # Full system fetch logic

base-src/src/sources/
  pluralkit.rs                    # PluralKitWebhookEvent type, event type filters

frontend/src/components/
  PluralKitWebhookConfig.vue      # Settings UI
```

---

## Security

- **Signing tokens:** Hashed with per-user salt before storage (SHA-256)
- **Constant-time comparison:** Prevents timing attacks
- **Unpredictable URLs:** System_id in URL acts as capability token
- **HTTPS required:** Webhook endpoint must use TLS

---

## Testing

1. **Unit tests:** Token validation, event type filtering
2. **Integration tests:** Mock PK webhook payloads → verify fetch triggered
3. **End-to-end:** Register webhook → trigger switch in PK → verify update propagates to targets

---

## Migration

1. Deploy database migration
2. Deploy webhook endpoint (initially disabled)
3. Deploy frontend UI (opt-in)
4. Monitor adoption via metrics

---

## Open Questions

None at this time — deferring coexistence/deduplication with SP WebSocket for later.

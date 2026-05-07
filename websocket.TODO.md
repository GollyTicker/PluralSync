# WebSocket Push Source — High-Level Spec

## 1. Overview

Add a new data source where external clients **push** fronting status updates via a persistent WebSocket connection, rather than the server pulling from SimplyPlural or PluralKit APIs. This enables custom integrations (e.g., a custom fronting tracker or third-party system) to feed fronting data into PluralSync's synchronization pipeline.

---

## 2. Deployment Feature Flag

The websocket push source is gated by a **deployment-level** feature flag, following the existing pattern of `discord_status_message_updater_available` in `ApplicationConfig`.

### `ApplicationConfig` additions

```rust
pub struct ApplicationConfig {
    // ... existing fields ...
    pub enable_websocket_push_source: bool,
}
```

- Read from environment variable `WEBSOCKET_PUSH_SOURCE_AVAILABLE`
- Defaults to `false`

### Behavior when flag is `false`

- The WebSocket route (`/api/user/platform/pluralsync/events`) **always exists** (no conditional route registration).
- The handler checks the flag on every connection attempt. If the flag is `false`, the handler returns **400 Bad Request** with JSON:
  ```json
  {"type":"error","result":"feature_disabled","data":"WebSocket push source is not available in this deployment"}
  ```
  The connection is then closed.
- No route-level or handler-level branching on the flag beyond this single check.

### Behavior when flag is `true`

- The handler proceeds normally (see §3 onward).

---

## 3. Protocol

| Direction | Message | Description |
|-----------|---------|-------------|
| Client → Server | `{"type":"login","user":"<email>","auth":"<jwt>"}` | Authenticate |
| Server → Client | `{"type":"login","result":"success","server_info":{"version":"<ver>"}}` | Ready for data |
| Server → Client | `{"type":"error","result":"<reason>","data":"<details>"}` then close | Auth denied |
| Client → Server | `{"type":"ping"}` | Keepalive |
| Server → Client | `{"type":"pong"}` | Keepalive response |
| Client → Server | `{"type":"fronters","data":<fronters_data>}` | Push fronter update |
| Server → Client | *(no response)* | Accepted (implicit ack) |
| Server → Client | `{"type":"fronters.response","result":"error","data":"<error>"}` | Invalid data (connection stays open) |

---

## 4. Authentication

- First `login` on a connection succeeds → binds the connection to that user.
- A **second** `login` message on the same connection is **always denied** (error JSON, then close), even if it authenticates a different user. This prevents hijacking or switching.
- JWT validated using the same `jsonwebtoken` HS256 logic as REST endpoints. The `sub` claim is the authoritative user identity.
- The `user` field (email) is informational only.
- If the user's source config is not `websocket` (i.e. it's SP or PK) → error JSON, then close.
- Source config changes are handled by the normal updater restart cycle — no "source_changed" error needed.

---

## 5. External API: `GenericFronter`

The `data` field in `{"type":"fronters","data":<data>}` is a JSON object:

```json
{
  "fronters": [
    {
      "id": "abc123",
      "name": "Alice",
      "pronouns": "she/her",
      "avatar_url": "https://...",
      "start_time": "2026-05-03T12:00:00Z",
      "privacy": "public"
    }
  ]
}
```

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | `string` | Yes | Stable unique identifier for this fronter |
| `name` | `string` | Yes | Display name |
| `pronouns` | `string` | No | Pronouns (e.g. `"she/her"`) |
| `avatar_url` | `string` | No | Avatar URL. Omit or empty string = no avatar |
| `start_time` | `string` | No | ISO 8601 UTC timestamp. If omitted, server uses current time |
| `privacy` | `"public"` or `"private"` | Yes | Binary privacy flag |

### Rules

- The `fronters` array represents the **complete current fronting state** — it replaces, not merges with, previous data.
- An empty array means "nobody is fronting."
- The server filters out any fronters with `privacy: "private"` before distributing to updaters.
- `pluralkit_id` and `privacy_buckets` are always `None`/`[]` internally (source-specific, not applicable to this generic source).

---

## 6. Error Handling

| Scenario | Response |
|----------|----------|
| Valid `fronters` | No response (implicit success) |
| Malformed JSON | `{"type":"fronters.response","result":"error","data":"<parse error>"}` — connection stays open |
| Missing required field | `{"type":"fronters.response","result":"error","data":"field 'X' is required"}` — connection stays open |
| Invalid `privacy` value | `{"type":"fronters.response","result":"error","data":"privacy must be 'public' or 'private'"}` — connection stays open |

**Note:** Unlike authentication failures (which close the connection), validation errors on `fronters` messages keep the connection open so the client can retry with corrected data. See §4.4 and §5 of the client spec.

---

## 7. Endpoint

```
GET /api/user/platform/pluralsync/events
```

A bare WebSocket upgrade route — **no Rocket `FromRequest` JWT guard**. Authentication is application-level, handled via the `login` message payload after the connection is established. Similar to how the SimplyPlural WebSocket connection works (no Rocket guard, auth is inside the protocol).

The route is **always registered** regardless of the deployment feature flag. The handler performs the flag check as the first step.

---

## 8. One Source Per User

Same exclusivity as SP/PK: a user has exactly one active source. A websocket connection is rejected if the user's config points to SP or PK. Config changes are handled by the normal updater restart cycle (the websocket is automatically stopped and restarted, just like all other updaters).

---

## 9. Frontend Integration

The frontend needs to know whether the websocket push source feature is available in the current deployment.

### Availability detection (changed from spec)

**Original plan:** The server's user info / startup metadata endpoint (e.g. `/api/user/info`) would include `websocket_push_source_available`.

**Current implementation:** The frontend uses hardcoded URL detection instead of a metadata endpoint. The `WebSocketConfigPanel` component checks `location.href` for known private/dev instance domains:

```ts
const websocketAvailable = ref(
  location.href.includes('https://private.pluralsync') ||
    location.href.includes('https://dev-online.pluralsync'),
)
```

When `websocketAvailable` is `false`, the entire config section is hidden — the `enable_from_websocket` checkbox is not displayed. No metadata endpoint field (`websocket_push_source_available`) was added.

### Config enforcement (UI)

When the feature flag is `false`, the UI never shows the websocket config section, so the user cannot toggle `enable_from_websocket` from the frontend. The server-side handler also enforces that `enable_from_websocket` must be `false` when the deployment flag is `false` (rejected via config update validation).

---

## 10. Architecture

```
External Client (custom fronting tracker)
       │
       │ WebSocket at /api/user/platform/pluralsync/events
       ▼
┌─────────────────────────────────┐
│ websocket.rs                     │
│                                  │
│ - Rocket WebSocket handler       │
│   (no JWT guard, no FromRequest) │
│ - Feature flag check (400 if     │
│   disabled)                      │
│ - Message parser (login/ping/    │
│   fronters)                      │
│ - JWT validation from message    │
│   payload                        │
│ - Fronter builder (JSON →        │
│   FilteredFronters)              │
│ - Send to fronter_channel        │
└─────────────┬───────────────────┘
              │
              ▼
┌─────────────────────────────────┐
│ FronterChannel                   │  (existing: rate-limited broadcast)
└─────────────┬───────────────────┘
              │
              ▼
┌─────────────────────────────────┐
│ ChangeProcessor                  │  (existing: no changes needed)
│ VRChat, Discord, ToPluralKit,    │
│ Website                          │
└─────────────────────────────────┘
```

### What's new

- **`src/plurality/websocket.rs`** — new module, route registered in `main.rs`. **Handler is a stub (`todo!()`)** — full implementation (message parser, JWT validation, Fronter builder, FronterChannel integration) is pending.
- **`ApplicationConfig.enable_websocket_push_source: bool`** — deployment-level feature flag (env: `WEBSOCKET_PUSH_SOURCE_AVAILABLE`, default `false`).
- **`src/plurality/websocket.rs` handler** — route registered at `GET /api/user/platform/pluralsync/events`. Handler body is `todo!()` pending implementation.
- **`enable_from_websocket: bool`** added to `UserConfigForUpdater` and `UserConfigDbEntries` (migration `025_add_websocket_source.sql`).
- **Source exclusivity validation** — `create_config_with_strong_constraints` enforces that only one source (SP, PK, or WebSocket) can be enabled at a time.
- **`WebSocketConfigPanel.vue`** — frontend config component, integrated into `ConfigSettings.vue`. Uses hardcoded URL detection for availability.
- **Integration tests** — `test/updater.ws.integration-tests.sh` with 9 test cases.
- **`metrics_config_values`** — `enable_from_websocket` metric added.
- **`AGENTS.md`** — updated with coding guidelines (not WebSocket-specific).

### What stays the same

- `FilteredFronters` struct (already source-agnostic)
- `Fronter` struct (already source-agnostic)
- `FronterChannel` with rate limiting (shared across all sources)
- `ChangeProcessor` and all updaters (no changes needed)
- `fetch_fronters()` — websocket data arrives push-style, never polled
- `websocket.spec.md` — client specification unchanged

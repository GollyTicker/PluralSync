# WebSocket Push Source — High-Level Spec

## 1. Overview

Add a new data source where external clients **push** fronting status updates via a persistent WebSocket connection, rather than the server pulling from SimplyPlural or PluralKit APIs. This enables custom integrations (e.g., a custom fronting tracker or third-party system) to feed fronting data into PluralSync's synchronization pipeline.

---

## 2. Protocol

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

## 3. Authentication

- First `login` on a connection succeeds → binds the connection to that user.
- A **second** `login` message on the same connection is **always denied** (error JSON, then close), even if it authenticates a different user. This prevents hijacking or switching.
- JWT validated using the same `jsonwebtoken` HS256 logic as REST endpoints. The `sub` claim is the authoritative user identity.
- The `user` field (email) is informational only.
- If the user's source config is not `websocket` (i.e. it's SP or PK) → error JSON, then close.
- Source config changes are handled by the normal updater restart cycle — no "source_changed" error needed.

---

## 4. External API: `GenericFronter`

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

## 5. Error Handling

| Scenario | Response |
|----------|----------|
| Valid `fronters` | No response (implicit success) |
| Malformed JSON | `{"type":"fronters.response","result":"error","data":"<parse error>"}` — connection stays open |
| Missing required field | `{"type":"fronters.response","result":"error","data":"field 'X' is required"}` — connection stays open |
| Invalid `privacy` value | `{"type":"fronters.response","result":"error","data":"privacy must be 'public' or 'private'"}` — connection stays open |

**Note:** Unlike authentication failures (which close the connection), validation errors on `fronters` messages keep the connection open so the client can retry with corrected data. See §4.4 and §6.5 of the spec.

---

## 6. Endpoint

```
GET /api/user/platform/pluralsync/events
```

A bare WebSocket upgrade route — **no Rocket `FromRequest` JWT guard**. Authentication is application-level, handled via the `login` message payload after the connection is established. Similar to how the SimplyPlural WebSocket connection works (no Rocket guard, auth is inside the protocol).

---

## 7. One Source Per User

Same exclusivity as SP/PK: a user has exactly one active source. A websocket connection is rejected if the user's config points to SP or PK. Config changes are handled by the normal updater restart cycle (the websocket is automatically stopped and restarted, just like all other updaters).

---

## 8. Architecture

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

- **`src/plurality/websocket.rs`** — new module for the WebSocket handler, message parser, JWT validation, and Fronter builder.
- **`start_updater()`** in `UpdaterManager` spawns the websocket listener task when `enable_from_websocket` is true.
- **`enable_from_websocket: bool`** added to `UserConfigForUpdater` and `UserConfigDbEntries`.

### What stays the same

- `FilteredFronters` struct (already source-agnostic)
- `Fronter` struct (already source-agnostic)
- `FronterChannel` with rate limiting (shared across all sources)
- `ChangeProcessor` and all updaters (no changes needed)
- `fetch_fronters()` — websocket data arrives push-style, never polled

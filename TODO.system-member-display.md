# Member Display Names

Per-member custom display names that flow through all output channels (fronting status, history, bridge, website).

## Key Design Decisions

- **Source-agnostic:** Works for SimplyPlural, PluralKit, and WebSocket sources. The `member_id` in the DB is the raw ID from whichever source the user has enabled (SP member ID, PK member ID, or websocket member ID).
- **Per-member config extensible:** The `member_display_names` table is the foundation for future per-member settings (e.g., per-member privacy overrides). The API uses `/api/fronting/members/{member_id}/name` not `/_display_name`.
- **Display name injected into `Fronter`:** The `Fronter` struct gets a `display_name: String` field. Custom names are injected at the conversion layer (when creating `Fronter` from source members), so all downstream code — status formatting, history, bridge, website — automatically uses the right name.
- **Privacy respected:** Display names are only returned for members the user is allowed to see (same privacy bucket / visibility rules).

## 1. Database

New table `member_display_names`:

```sql
CREATE TYPE member_source_enum AS ENUM ('simply_plural', 'pluralkit', 'websocket');

CREATE TABLE member_display_names (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    member_id TEXT NOT NULL,
    source member_source_enum NOT NULL,
    display_name TEXT NOT NULL,
    PRIMARY KEY (user_id, member_id)
);
```

- `ON CONFLICT (user_id, member_id) DO UPDATE` for upserts
- `ON CONFLICT (user_id, member_id) DO DELETE` for clearing a display name back to default

## 2. Backend

### 2a. Database queries

New file: `src/database/member_display_names.rs`

- `get_member_display_names(db_pool, user_id) -> Result<HashMap<(MemberSource, String), String>>` — returns `((source, member_id), display_name)`
- Define `MemberSource` Rust enum matching the SQL enum: `SimplyPlural`, `PluralKit`, `Websocket`
- `set_member_display_name(db_pool, user_id, source, member_id, display_name) -> Result<()>` — upsert
- `delete_member_display_name(db_pool, user_id, source, member_id) -> Result<()>` — clear to default

### 2b. Inject display names into Fronter

Modify the conversion from source members to `Fronter`:

- **SimplyPlural** (`src/plurality/simply_plural.rs`): When converting `Member` → `Fronter`, look up the custom display name from the fetched map. Use it if present, otherwise use the member's canonical `name`.
- **PluralKit** (`src/plurality/pluralkit.rs`): Already has `m.display_name` from the API. Extend to also check the custom display name map (custom name overrides PK's own display_name).
- Add `display_name: String` field to the `Fronter` struct in `src/plurality/model.rs`.

The `name` field stays as the canonical name (useful for debugging). The `display_name` field is what gets shown everywhere.

### 2c. API endpoints

New file: `src/plurality/member_display_api.rs`

- `GET /api/fronting/members` — return the list of members (from the active source) merged with custom display names. Respects privacy rules (hidden members are excluded).
- `PUT /api/fronting/members/{member_id}/name` — update a member's display name. Triggers `fetch_and_update_fronters` on the user's updater to refresh status immediately.
- `DELETE /api/fronting/members/{member_id}/name` — clear a member's display name back to default. Also triggers `fetch_and_update_fronters`.

The `source` is determined from the user's config (which source is enabled).

### 2d. Register routes

Mount in `src/main.rs` routes. Add module to `src/plurality/mod.rs`.

## 3. Frontend

New page: `frontend/src/components/MemberDisplayNameConfig.vue`

- **Virtual scrolling** for the member list (not pagination) — handles large member lists efficiently on all devices
- **Search bar** at top — filters by canonical name or display name in real-time
- Each member shows their name and an editable display name input
- **Debounce** (e.g., 500ms) before saving to avoid excessive API calls
- Empty display name = cleared (sends DELETE)
- Route: `/config/members` (under config section)
- Navigation link added in `App.vue`

API functions in `pluralsync_api.ts`:
- `get_fronting_members()` → `MemberDisplayInfo[]`
- `set_member_name(member_id: string, name: string)` → `void`
- `delete_member_name(member_id: string)` → `void`

## 4. Testing

- DB queries: upsert, delete, empty results
- API: auth required, privacy filtering, input validation, conflict handling
- Fronter injection: custom name used when set, fallback to canonical when not
- End-to-end: update display name → `fetch_and_update_fronters` called → status text reflects change

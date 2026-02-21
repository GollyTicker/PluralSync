# History

The users shall be able to see two kinds of history in the History tab.

1. History of fronting statuses
2. History of changes/diffs in the system

---

## Implementation Summary: Fronting Status History

### Overview
Store each fronting change as a formatted status text. Users can view the last X statuses in a new History tab. The history automatically prunes after X events, where X is user-configurable (0–1000; default 100). And after N days with 0 <= N <= 30 (default 7).

### Database Schema
**Table: `history_status`**
```sql
CREATE TABLE history_status (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status_text TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### Backend Implementation

**1. Config Extension (`src/users/config.rs`)**
- Add `history_limit: Option<i32>` to `UserConfigDbEntries`
- Default value: `Some(100)`
- Validation: 0–1000 range

**2. History Module (`src/history/mod.rs`)**
- `HistoryEntry` struct with `specta::Type` export:
  - `id: String`
  - `user_id: UserId`
  - `status_text: String`
  - `created_at: chrono::DateTime<chrono::Utc>`
- Storage functions:
  - `store_history_entry(pool, user_id, status_text)` – inserts new entry, then prunes old entries beyond limit
  - `get_history_entries(pool, user_id, limit)` – returns latest N entries ordered by created_at DESC
- Deduplication: Only store if `status_text` differs from most recent entry

**3. API Endpoint**
- `GET /api/user/history/fronting`
- Returns `Vec<HistoryEntry>`
- Requires JWT authentication

**4. Integration Point (`src/updater/manager.rs`)**
In `fetch_and_update_fronters()`, after sending fronters to channel:
- Generate formatted status string using existing `format_fronting_status()`
- Check if different from last stored entry (deduplication)
  - This can be done with the existing `OnlyChangesImmediateSend`
- Call `store_history_entry()` to persist

### Frontend Implementation

**1. New Component (`frontend/src/components/HistoryTab.vue`)**
- Fetches history via `GET /api/user/history/fronting`
- Displays as timeline/list with:
  - Status text (e.g., "F: Ania, Björn")
  - Timestamp formatted as relative time ("5 min ago", "2 hours ago")
- Styled similar to `StatusDisplay.vue` example text
  - Refactor both to make visuals consistent and reuse the same logic

**2. Navigation (`frontend/src/App.vue`)**
- Add router link: `<router-link v-if="loggedIn" to="/history">History</router-link>`

**3. Route (`frontend/src/router.ts`)**
- Add route: `{ path: '/history', component: HistoryTab }`

**4. Config UI (`frontend/src/components/ConfigSettings.vue` or new panel)**
- Add input for `history_limit`, `history_truncate_after`. History is shown as "disabled", when the limit is 0 entries or 0 days.
- Save via existing config update mechanism

### Pruning Strategy
After inserting a new history entry:
```sql
DELETE FROM history_status
WHERE user_id = $1
  AND (
    -- Prune by count: keep only the most recent N entries
    id NOT IN (
      SELECT id FROM history_status
      WHERE user_id = $1
      ORDER BY time DESC
      LIMIT $2
    )
    OR
    -- Prune by age: remove entries older than N days
    time < NOW() - ($3 || ' days')::INTERVAL
  );
```

**Parameters:**
- `$1` = user UUID
- `$2` = history limit (number of entries to keep)
- `$3` = number of days to keep history

**Notes:**
- If limit is 0, all entries are pruned (disables history)
- If days is 0, no age-based pruning occurs
- Both conditions are OR'd so entries are removed if they exceed EITHER limit

### Files to Create/Modify
**Create:**
- `src/history/mod.rs`
- `frontend/src/components/HistoryTab.vue`

**Modify:**
- `docker/migrations/015_history_status.sql` – updated schema
- `src/users/config.rs` – add `history_limit` field
- `src/updater/manager.rs` – integrate history storage
- `src/main.rs` – register new endpoint
- `frontend/src/router.ts` – add history route
- `frontend/src/App.vue` – add navigation link
- `frontend/src/components/ConfigSettings.vue` – add history_limit config
- `frontend/src/pluralsync_api.ts` – add API call for history
- `frontend/src/pluralsync.bindings.ts` – auto-generated from specta

### Notes
- Start simple: only store `status_text`, no fronter IDs
- Use existing formatting logic (`format_fronting_status`) for consistency
- Automatic pruning keeps storage bounded per user
- History limit of 0 effectively disables history tracking

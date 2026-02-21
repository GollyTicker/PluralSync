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

**1. Config Extension (`src/users/config.rs`)** ✅ COMPLETED
- Add `history_limit: Option<i32>` to `UserConfigDbEntries`
- Add `history_truncate_after_days: Option<i32>` to `UserConfigDbEntries`
- Default values: `Some(100)` and `Some(7)`
- Validation: 0–1000 range for history_limit, 0–30 for history_truncate_after_days

**2. History Module (`src/history/history.rs`)** ✅ COMPLETED
- `HistoryEntry` struct with `specta::Type` export:
  - `id: String`
  - `user_id: UserId`
  - `status_text: String`
  - `created_at: chrono::DateTime<chrono::Utc>`
- Storage functions:
  - `store_history_entry(pool, user_id, status_text, history_limit, history_truncate_after_days)` – inserts new entry, then prunes old entries beyond limit
  - `get_history_entries(pool, user_id, limit)` – returns latest N entries ordered by created_at DESC
  - `get_most_recent_status_text(pool, user_id)` – for deduplication
  - `prune_history(pool, user_id, history_limit, history_truncate_after_days)` – internal pruning logic
- Deduplication: Only store if `status_text` differs from most recent entry (to be integrated in manager.rs)

**3. API Endpoint** ⏳ TODO
- `GET /api/user/history/fronting`
- Returns `Vec<HistoryEntry>`
- Requires JWT authentication

**4. Integration Point (`src/updater/manager.rs`)** ⏳ TODO
In `fetch_and_update_fronters()`, after sending fronters to channel:
- Generate formatted status string using existing `format_fronting_status()`
- Check if different from last stored entry (deduplication)
  - This can be done with the existing `OnlyChangesImmediateSend`
- Call `store_history_entry()` to persist

### Frontend Implementation

**1. New Component (`frontend/src/components/HistoryTab.vue`)** ⏳ TODO
- Fetches history via `GET /api/user/history/fronting`
- Displays as timeline/list with:
  - Status text (e.g., "F: Ania, Björn")
  - Timestamp formatted as relative time ("5 min ago", "2 hours ago")
- Styled similar to `StatusDisplay.vue` example text
  - Refactor both to make visuals consistent and reuse the same logic

**2. Navigation (`frontend/src/App.vue`)** ⏳ TODO
- Add router link: `<router-link v-if="loggedIn" to="/history">History</router-link>`

**3. Route (`frontend/src/router.ts`)** ⏳ TODO
- Add route: `{ path: '/history', component: HistoryTab }`

**4. Config UI (`frontend/src/components/ConfigSettings.vue` or new panel)** ⏳ TODO
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
      ORDER BY created_at DESC
      LIMIT $2
    )
    OR
    -- Prune by age: remove entries older than N days
    created_at < NOW() - ($3 || ' days')::INTERVAL
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
**Create:** ✅ COMPLETED
- `src/history/mod.rs`
- `src/history/history.rs`

**Modify:** 
- `docker/migrations/015_history_status.sql` – updated schema ✅ COMPLETED
- `src/users/config.rs` – add `history_limit` and `history_truncate_after_days` fields ✅ COMPLETED
- `src/lib.rs` – register history module ✅ COMPLETED
- `src/users/model.rs` – add `specta::Type` to `UserId` ✅ COMPLETED
- `src/database/queries.rs` – include new fields in queries ✅ COMPLETED
- `src/database/constraints.rs` – include new fields in downgrade/upgrade functions ✅ COMPLETED
- `src/updater/manager.rs` – integrate history storage ⏳ TODO
- `src/main.rs` – register new endpoint ⏳ TODO
- `src/users/history_api.rs` – create API endpoint ⏳ TODO
- `frontend/src/router.ts` – add history route ⏳ TODO
- `frontend/src/App.vue` – add navigation link ⏳ TODO
- `frontend/src/components/HistoryTab.vue` – create component ⏳ TODO
- `frontend/src/components/ConfigSettings.vue` – add history_limit config ⏳ TODO
- `frontend/src/pluralsync_api.ts` – add API call for history ⏳ TODO
- `frontend/src/pluralsync.bindings.ts` – auto-generated from specta ⏳ TODO

### Notes
- Start simple: only store `status_text`, no fronter IDs
- Use existing formatting logic (`format_fronting_status`) for consistency
- Automatic pruning keeps storage bounded per user
- History limit of 0 effectively disables history tracking

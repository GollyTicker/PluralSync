# Text-based System Changelog

Users should be able to view a rough overview of the latest system changes in a simple text-based diff format.

The format should be easily readable for the general public, because they are not used to the green/red diff view.

The diff should be based on changes in the CustomFronts + Members dump of the system.

## Trigger Mechanism

The existing `fetch_fronts()` function already fetches members and custom fronts from SimplyPlural.
Extend this to also capture the full `MemberContent` and `CustomFrontContent` JSON data for changelog processing.

**Integration:** Call `compute_and_store_changelog()` after fetching the system snapshot.
No separate "full fetch" task needed—reuse what we already fetch.

## Storage Model

- **Latest snapshot:** Stored in `system_changelog_snapshot` table to compute new deltas. Replaced on each update.
- **History:** Only **deltas** are stored in `system_changelog` table.
- **Per-system changelog** (not global).
- **No backfill** when feature is first enabled—only track changes going forward.
- **Automatic initialization:** When snapshot is accessed for the first time, store current system state as baseline and report no diffs.
- Retention: Only the last X entries and N days are stored. Configuration via `changelog_limit` and `changelog_truncate_after_days` columns in `users` table (equivalent to history fronting).

## Data Structures

### Snapshot Format

```rust
pub struct SystemSnapshot {
    // Map: member_id → MemberContent JSON (enriched with PluralKit names)
    pub members: HashMap<String, serde_json::Value>,
    // Map: custom_front_id → CustomFrontContent JSON
    pub custom_fronts: HashMap<String, serde_json::Value>,
    // Map: bucket_id → bucket name
    pub privacy_buckets: HashMap<String, String>,
    // System metadata
    pub system_uid: String,
    pub system_name: String,
    pub fetched_at: chrono::DateTime<chrono::Utc>,
}
```

**Key:** Store `MemberContent`/`CustomFrontContent` (not wrapper structs), with ID at top level in delta arrays.

**Enrichment:** PluralKit member names are fetched and added directly to `MemberContent` JSON (e.g., `pk_name`, `pk_display_name` fields) before storing. This way they're included in the diff automatically.

### Changelog Entry Format

```rust
pub struct ChangelogEntry {
    pub id: Uuid,
    pub user_id: UserId,
    pub timestamp: DateTime<Utc>,

    // Delta sets (arrays of content objects with ID at top level)
    pub members_added: Option<serde_json::Value>,        // [{id, ...MemberContent fields}, ...]
    pub members_removed: Option<serde_json::Value>,      // [{id, ...MemberContent fields}, ...]
    pub members_modified: Option<serde_json::Value>,     // [{id, old, new}, ...]
    pub custom_fronts_added: Option<serde_json::Value>,  // [{id, ...CustomFrontContent fields}, ...]
    pub custom_fronts_removed: Option<serde_json::Value>,// [{id, ...CustomFrontContent fields}, ...]
    pub custom_fronts_modified: Option<serde_json::Value>,// [{id, old, new}, ...]

    // System metadata delta
    pub system_uid: String,                              // Always present, never changes
    pub old_system_name: Option<String>,                 // Previous system name (None on first snapshot)
    pub new_system_name: String,                         // Current system name

    // Privacy buckets delta
    pub old_privacy_buckets: Option<serde_json::Value>,  // {bucket_id: name, ...}
    pub new_privacy_buckets: serde_json::Value,          // {bucket_id: name, ...}
}

pub struct ModifiedItem {
    pub id: String,
    pub old: serde_json::Value,  // Full MemberContent/CustomFrontContent
    pub new: serde_json::Value,
}
```

**Same structs used everywhere:** Database storage (`FromRow`), backend processing, frontend API response (`Serialize`).

## Delta Computation

Given stored snapshot A and new snapshot B:

1. **Members added:** IDs in B but not in A → store full `MemberContent` with ID at top level
2. **Members removed:** IDs in A but not in B → store full `MemberContent` with ID at top level
3. **Members modified:** IDs in both A and B where JSON differs → store `{id, old, new}`
4. **Same logic for custom fronts**
5. **System name changed:** If `A.system_name != B.system_name` → store both old and new
6. **Privacy buckets changed:** Store full old and new maps
7. **Replace snapshot A with B** in database

### Truncation & Hashing

- Apply **before** delta computation (truncate both A and B)
- All potentially unbounded string fields truncated if > 10,000 characters
- Format: `"[truncated long field (#<6_hex_chars>] <first_10k_chars> [truncated]"`
- SHA-256 for deterministic, stable hashing across platforms
- Use first 6 hex characters of hash (3 bytes)
- Recursive: applies to all string fields in nested JSON

Example:
```
Original: "This is a very long description..." (15,000 chars)
Truncated: "[truncated long field (#a1b2c3] This is a very long description...[first 10k chars]... [truncated]"
```

## Field Lookups (at Fetch Time)

Lookups happen **at fetch time** so changelog is self-contained and doesn't degrade if members are deleted.

### PluralKit Member Names (Enrichment)

- For each member with `pluralkit_id`, fetch `GET /members/{pk_id}` from PluralKit API
- Add `pk_name` and `pk_display_name` fields directly to `MemberContent` JSON
- These enriched fields are then included in the diff automatically
- **Note:** Rate limiting and caching to be added later. For now, fetch all needed members.

### System Metadata

- Fetch system info from SimplyPlural API
- Store `system_uid` (always present, never changes)
- Store `system_name` (may change over time, track old/new in changelog)

### Privacy Buckets

- Fetch bucket definitions from SimplyPlural API
- Store as `Map<bucket_id, bucket_name>` in snapshot
- Track delta: `old_privacy_buckets` → `new_privacy_buckets` in changelog entry

**Fail-fast:** Any lookup error aborts the entire changelog update (no partial fills).

## Database Schema

```sql
-- One row per system-level change
CREATE TABLE system_changelog (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Delta sets (arrays stored as JSONB)
    members_added JSONB DEFAULT '[]'::jsonb,
    members_removed JSONB DEFAULT '[]'::jsonb,
    members_modified JSONB DEFAULT '[]'::jsonb,
    custom_fronts_added JSONB DEFAULT '[]'::jsonb,
    custom_fronts_removed JSONB DEFAULT '[]'::jsonb,
    custom_fronts_modified JSONB DEFAULT '[]'::jsonb,

    -- System metadata delta
    system_uid TEXT NOT NULL,
    old_system_name TEXT,
    new_system_name TEXT NOT NULL,

    -- Privacy buckets delta
    old_privacy_buckets JSONB,
    new_privacy_buckets JSONB NOT NULL
);

CREATE INDEX idx_system_changelog_user_timestamp
    ON system_changelog(user_id, timestamp DESC);

-- Latest full snapshot (for computing deltas)
CREATE TABLE system_changelog_snapshot (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    snapshot JSONB NOT NULL,
    fetched_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- User configuration for changelog retention
ALTER TABLE users
ADD COLUMN changelog_limit INTEGER CHECK (changelog_limit >= 0) DEFAULT 100,
ADD COLUMN changelog_truncate_after_days INTEGER CHECK (changelog_truncate_after_days >= 0) DEFAULT 30;
```

## HTTP Efficiency

**Combined fetch:** `fetch_fronts_and_system_snapshot()` returns both `(Vec<Fronter>, SystemSnapshot)`:

1. `GET /fronters` → front entries
2. `GET /members/{system_id}` → all members
3. `GET /customFronts/{system_id}` → all custom fronts (if enabled)
4. `GET /customFields/{system_id}` → VRChat status field ID (existing)
5. `GET /systems/{system_id}` → system metadata (name, uid)
6. `GET /privacyBuckets/{system_id}` → privacy bucket definitions
7. `GET /members/{pk_id}` → PluralKit member info (for each member with pk_id)

**Total:** Same core HTTP requests as before, plus system metadata and bucket lookups. PluralKit lookups are one per member with pk_id.

## Frontend Display

- The history tab already exists and shows fronting history.
- Add the **system changelog** in the same location.
- **Fetch entire history, ignore pagination** for simplicity.
- Displayed **nearby** fronting history, but **strictly separate features**.
- Frontend computes field-level changes from `old`/`new` JSON for display.
- Display format (human-readable, not programmer diff):
  - **Added:** `+ Member: "Name" (PK: KitName)`
  - **Removed:** `- CustomFront: "Name"`
  - **Modified:** `~ Member: "Name" - name (Old → New), description ([truncated long field (#a1b2c3] first 10k... [truncated] → [truncated long field (#def456] first 10k... [truncated]))`
  - **System name changed:** `~ System: "Old Name" → "New Name"`
  - **Privacy bucket changed:** `+ Bucket: "Bucket Name"` or `- Bucket: "Bucket Name"`
- Frontend UI layout details to be determined later.

## Module Structure

```
src/
├── changelog/
│   ├── mod.rs           # Public exports
│   ├── model.rs         # SystemSnapshot, ChangelogEntry, ModifiedItem
│   ├── delta.rs         # compute_delta(), compute_field_changes()
│   ├── cache.rs         # PluralKitCache (for later)
│   ├── lookups.rs       # fetch_pluralkit_lookups(), fetch_system_metadata(), fetch_privacy_bucket_lookups()
│   ├── storage.rs       # store_changelog_entry(), load_or_initialize_snapshot(), prune_changelog()
│   └── changelog_api.rs # GET /api/user/history/changelog
├── plurality/
│   ├── truncation.rs    # truncate_and_hash()
│   └── simply_plural.rs # fetch_fronts_and_system_snapshot() (new function)
└── database/
    └── changelog_queries.rs
```

## Implementation Checklist

- [ ] Create migration `docker/migrations/017_system_changelog.sql`
- [ ] Add `truncate_and_hash()` function in `src/plurality/truncation.rs`
- [ ] Add `SystemSnapshot` and `ChangelogEntry` structs in `src/changelog/model.rs`
- [ ] Implement `compute_delta()` in `src/changelog/delta.rs`
- [ ] Implement lookup functions in `src/changelog/lookups.rs`
- [ ] Implement storage functions in `src/changelog/storage.rs`
- [ ] Add `fetch_fronts_and_system_snapshot()` in `src/plurality/simply_plural.rs`
- [ ] Add `GET /api/user/history/changelog` endpoint in `src/changelog/changelog_api.rs`
- [ ] Integrate changelog processing in `src/updater/manager.rs` (`fetch_and_update_fronters`)
- [ ] Add frontend display in history tab (frontend/)

# Source-Agnostic Updates - Refined Architecture

## Design Decisions (Resolved)

### 1. Fronter Model

```rust
/// Distinguishes between member fronts and custom fronts for rendering purposes.
/// This matters because custom fronts (e.g., "overstimulated") convey different
/// information than member fronts (e.g., "Annalena").
pub enum Fronter {
    /// A system member from a source (SP or PK)
    Member {
        member_id: String,
        data: FronterData,
    },
    /// A custom front (SimplyPlural-specific concept)
    CustomFront {
        custom_front_id: String,
        data: FronterData,
    },
}

/// Shared data for all fronter types
pub struct FronterData {
    pub name: String,
    pub pronouns: Option<String>,
    pub avatar_url: Option<String>,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub source: SourcePlatform,  // Tracks which source provided this data
    /// Source-specific metadata (e.g., PluralKit ID for lookup)
    pub metadata: FronterMetadata,
}

pub struct FronterMetadata {
    /// PluralKit member ID (for PK source or SP→PK enrichment)
    pub pluralkit_member_id: Option<String>,
    /// SimplyPlural-specific VRChat status name
    pub vrchat_status_name: Option<String>,
    /// Privacy buckets (SP) or field privacy flags (PK)
    pub privacy: PrivacyInfo,
}
```

**Key Decisions:**
- `Fronter` is an **enum** to distinguish Member vs CustomFront at type level
- CustomFronts are **SP-only**; PK source never produces CustomFront variants
- `start_time` is **optional** - ordering strategy handles missing values
- Fronter equality: **`fronter_id` (member_id or custom_front_id) determines equality**
- Cross-source ID collisions **must be avoided** (implementation responsibility)

---

### 2. Front Structure & Ordering

```rust
/// Always uses Vec for flexibility; ordering applied separately
pub struct Front {
    pub fronters: Vec<Fronter>,
    pub ordering: FronterOrdering,
}

/// User-configurable ordering strategies (applied during rendering)
pub enum FronterOrdering {
    /// Primary sort by start_time; ties broken by member_created_at
    ByStartTime { reverse: bool },
    /// Alphabetical by name
    Alphabetical { reverse: bool },
    /// By member creation date
    ByMemberCreatedAt { reverse: bool },
    /// Preserve source-provided order (SP default, PK switch order)
    SourceOrder,
}

impl Front {
    /// Apply ordering and return sorted fronters
    pub fn ordered_fronters(&self) -> Vec<&Fronter> {
        // Implementation applies self.ordering to self.fronters
    }
}
```

**Key Decisions:**
- **Always `Vec<Fronter>`** - no `HashSet`, no `Unordered` variant
- Ordering is **separate from source** - user configured per target
- Sources return `Vec<Fronter>` in their native order
- Targets apply `FronterOrdering` during rendering

---

### 3. Source Configuration

```rust
pub struct SourceConfig {
    /// Primary source (required) - SP or PK
    pub primary: SourcePlatform,
    
    /// Enrichment source (optional) - provides additional data
    pub enrichment: Option<SourcePlatform>,
    
    /// Per-field sourcing priority
    pub field_sourcing: FieldSourcingConfig,
}

pub enum SourcePlatform {
    SimplyPlural,
    PluralKit,
    // "Manual" removed per requirements
}

/// Per-field sourcing configuration
pub struct FieldSourcingConfig {
    /// Which source to prefer for member names
    pub name: FieldSource,
    /// Which source to prefer for pronouns
    pub pronouns: FieldSource,
    /// Which source to prefer for start_time
    pub start_time: FieldSource,
    /// Which source to prefer for avatar URLs
    pub avatar: FieldSource,
}

/// Field-level source priority
pub enum FieldSource {
    /// Always use primary source
    Primary,
    /// Always use enrichment source (if available)
    Enrichment,
    /// Try primary first; fall back to enrichment if missing/empty
    PrimaryThenEnrichment,
    /// Try enrichment first; fall back to primary if missing/empty
    EnrichmentThenPrimary,
}
```

**Key Decisions:**
- **Per-field priority is highest** - overrides general primary/enrichment
- Minimum configurable fields: `name`, `pronouns`, `start_time`
- Extended fields: `avatar`, and potentially others
- **"Manual" source removed** from scope

---

### 4. Source Trait

```rust
/// Abstraction for fronting data sources
#[async_trait]
pub trait Source {
    /// Fetch current front from this source
    /// Returns Vec<Fronter> in source-native order
    async fn fetch_front(&self, config: &SourceCredentials) -> Result<Vec<Fronter>>;
    
    /// Subscribe to real-time changes (WebSocket, webhooks, etc.)
    /// Returns a channel that yields updates
    async fn subscribe_changes(
        &self,
        config: &SourceCredentials,
    ) -> Result<LatestReceiver<Vec<Fronter>>>;
    
    /// Source platform identifier
    fn platform(&self) -> SourcePlatform;
}

/// Credentials needed for a specific source
pub enum SourceCredentials {
    SimplyPlural { api_token: Decrypted },
    PluralKit { api_token: Decrypted },
}
```

**Key Decisions:**
- Returns **`Vec<Fronter>`** - ordering applied by targets
- Sources **don't need user ordering preference**
- Assumes all sources have **some push mechanism** (WebSocket/webhook)
- PK webhooks **not implemented yet** - polling fallback for now
- Credentials handled via existing `UserConfigForUpdater` secrets

---

### 5. Database Schema

```sql
-- New columns for user_config table
ALTER TABLE user_config ADD COLUMN primary_source_platform VARCHAR(50) DEFAULT 'simply_plural';
ALTER TABLE user_config ADD COLUMN enrichment_source_platform VARCHAR(50) DEFAULT NULL;

-- Fronter ordering configuration
ALTER TABLE user_config ADD COLUMN fronter_ordering VARCHAR(50) DEFAULT 'by_start_time';
ALTER TABLE user_config ADD COLUMN fronter_ordering_reverse BOOLEAN DEFAULT FALSE;

-- Per-field sourcing (separate columns for type safety)
ALTER TABLE user_config ADD COLUMN name_source VARCHAR(50) DEFAULT 'primary';
ALTER TABLE user_config ADD COLUMN pronouns_source VARCHAR(50) DEFAULT 'primary';
ALTER TABLE user_config ADD COLUMN start_time_source VARCHAR(50) DEFAULT 'primary';
ALTER TABLE user_config ADD COLUMN avatar_source VARCHAR(50) DEFAULT 'primary';

-- Privacy configuration for multi-source
ALTER TABLE user_config ADD COLUMN respect_member_field_privacy BOOLEAN DEFAULT TRUE;
-- Note: privacy_fine_grained and privacy_fine_grained_buckets remain SP-specific
```

**Key Decisions:**
- **Many columns is okay** for per-field sourcing (type-safe, queryable)
- Users **can't select PK as primary if `enable_to_pluralkit = false`**
- PK field privacy: **fetch all, filter during rendering** based on config
- Default: **respect member field privacy**

---

### 6. Custom Fronts Limitation

```rust
/// Custom fronts are SP-only
impl PluralKitSource {
    async fn fetch_front(&self, config: &SourceCredentials) -> Result<Vec<Fronter>> {
        // PK only returns Member variants
        // Custom fronts are NOT supported when PK is primary
        Ok(members.into_iter().map(Fronter::Member).collect())
    }
}

impl SimplyPluralSource {
    async fn fetch_front(&self, config: &SourceCredentials) -> Result<Vec<Fronter>> {
        // SP returns both Member and CustomFront variants
        Ok(members.into_iter().map(Fronter::Member)
            .chain(custom_fronts.into_iter().map(Fronter::CustomFront))
            .collect())
    }
}
```

**Key Decisions:**
- **CustomFronts only when SP is primary**
- If PK is primary + SP is enrichment: custom fronts are **ignored**
- No conversion of custom fronts to "pseudo-members"

---

### 7. Bridge Compatibility

**No changes required:**
- Bridge receives **rendered `DiscordRichPresence`** via SSE
- Bridge doesn't care about source model
- WebSocket/SSE message format **unchanged**
- `ServerToBridgeSseMessage` structure remains the same

---

### 8. Error Handling

```rust
/// Fetch from multiple sources in parallel
async fn fetch_from_sources(
    primary: &dyn Source,
    enrichment: Option<&dyn Source>,
    primary_creds: &SourceCredentials,
    enrichment_creds: Option<&SourceCredentials>,
) -> Result<(Vec<Fronter>, Option<Vec<Fronter>>)> {
    // Fail-fast: both sources must succeed
    // If primary fails → error
    // If enrichment fails (when configured) → error
    tokio::try_join!(
        primary.fetch_front(primary_creds),
        async {
            match enrichment {
                Some(src) => src.fetch_front(enrichment_creds.unwrap()).await,
                None => Ok(vec![]),
            }
        }
    )
}
```

**Key Decisions:**
- **Fail-fast**: both primary and enrichment must succeed
- No fallback to partial data
- Error logged, updater status set to `Error(message)`

---

### 9. Metrics

```rust
// Per-source metrics (in addition to existing per-updater metrics)
int_counter_metric!(SOURCE_FETCH_REQUESTS_TOTAL, &["source", "user_id"]);
int_counter_metric!(SOURCE_FETCH_FAILURES_TOTAL, &["source", "user_id"]);
int_gauge_metric!(SOURCE_FETCH_LATENCY_MS, &["source", "user_id"]);
int_gauge_metric!(SOURCE_FRONTERS_COUNT, &["source", "user_id"]);
```

**Key Decisions:**
- **Per-source metrics** required for monitoring
- Track: requests, failures, latency, fronter count
- Separate from existing per-updater metrics

---

## Implementation Plan (Revised)

### Phase 1: Foundation (`base-src`)
- [ ] Create `Fronter` enum (Member/CustomFront)
- [ ] Create `FronterData` struct
- [ ] Create `FronterMetadata` struct
- [ ] Create `Front` struct (Vec + ordering)
- [ ] Create `FronterOrdering` enum
- [ ] Create `SourcePlatform` enum (SP, PK only)
- [ ] Update `FrontingFormat` to accept `Front` instead of `&[Fronter]`
- [ ] Export new types via `base-src/src/lib.rs`

### Phase 2: Source Abstraction
- [ ] Create `Source` trait in `src/plurality/sources/`
- [ ] Create `SourceCredentials` enum
- [ ] Implement `SimplyPluralSource` adapter
  - [ ] Returns `Vec<Fronter>` with both Member and CustomFront
  - [ ] Preserves SP-native order
- [ ] Implement `PluralKitSource` adapter
  - [ ] Returns `Vec<Fronter>` with Member only
  - [ ] Preserves PK switch order
- [ ] Update `change_processor.rs` to use source abstraction
  - [ ] Replace direct SP calls with `Source::fetch_front()`

### Phase 3: Enrichment & Merging
- [ ] Implement parallel fetching with `tokio::try_join!`
- [ ] Implement per-field merging logic
  - [ ] Match fronters by ID across sources
  - [ ] Apply `FieldSourcingConfig` per field
  - [ ] Handle missing fields gracefully
- [ ] Add `FieldSourcingConfig` to user config

### Phase 4: Configuration & Database
- [ ] Add new config columns (see schema above)
- [ ] Add per-source metrics
- [ ] Update `UserConfigDbEntries` struct
- [ ] Update `UserConfigForUpdater` struct
- [ ] Update config API endpoints (`/api/user/config`)
- [ ] Add validation: PK primary requires `enable_to_pluralkit = true`

### Phase 5: Frontend Settings UI
- [ ] Source selection dropdown (primary + enrichment)
- [ ] Per-field sourcing matrix UI
- [ ] Ordering configuration (dropdown + reverse checkbox)
- [ ] Validation: show warnings for unsupported combinations
  - [ ] PK primary + SP enrichment → custom fronts ignored
  - [ ] PK primary → no custom front support

### Phase 6: Privacy & Rendering
- [ ] Implement PK field privacy filtering
- [ ] Update rendering logic to respect `respect_member_field_privacy`
- [ ] Update all targets (VRChat, Discord, PK, Website) to use new `Front` type
- [ ] Apply `FronterOrdering` during rendering per target

### Phase 7: Cleanup & Migration
- [ ] Add database migration for new columns
- [ ] Migrate existing users:
  - [ ] `primary_source_platform = 'simply_plural'` (default)
  - [ ] `enrichment_source_platform = NULL` (default)
  - [ ] All field sources = `'primary'` (default)
- [ ] Remove legacy SP-specific code paths where appropriate
- [ ] Update documentation
- [ ] Run TypeScript binding generation

---

## Open Questions (Deferred)

1. **Rate limit coordination** - How to handle combined rate limits when both sources are active?
2. **Testing strategy** - Mock sources vs. integration tests with real APIs?
3. **PK webhook support** - Future enhancement for real-time PK updates
4. **Manual source** - Explicitly out of scope for now

---

## Migration Notes

### Backward Compatibility
- Existing SP-only users: **no breaking changes**
- Default config: `primary = SimplyPlural`, no enrichment
- All existing updaters continue working

### Breaking Changes
- `FrontingFormat::format_fronting_status()` signature changes
  - Old: `format_fronting_status(&FrontingFormat, &[Fronter])`
  - New: `format_fronting_status(&FrontingFormat, &Front)`
- Bridge: **no changes required**

### Rollout Strategy
1. Deploy database migration first
2. Deploy backend with new code (backward-compatible defaults)
3. Deploy frontend with new settings UI
4. Users can opt-in to new features gradually

---

## File Structure (New)

```
base-src/src/
  platforms.rs          # Existing (Discord RPC types)
  updater.rs            # Existing (UpdaterStatus)
  
  # New types for source-agnostic:
  sources/
    mod.rs
    fronter.rs          # Fronter enum, FronterData, FronterMetadata
    front.rs            # Front struct, FronterOrdering
    platform.rs         # SourcePlatform enum

src/
  plurality/
    # Existing:
    simply_plural.rs
    simply_plural_model.rs
    fronting_status.rs
    
    # New:
    sources/
      mod.rs
      trait.rs          # Source trait
      simply_plural_source.rs
      pluralkit_source.rs
      credentials.rs    # SourceCredentials enum
    
    # Updated:
    fronting_status.rs  # Now accepts Front instead of &[Fronter]

  users/
    config.rs           # Add new source config fields
```

---

## Next Steps

1. **Review this architecture** - Confirm all decisions are captured correctly
2. **Create Phase 1 PR** - Foundation types in `base-src`
3. **Draft database migration** - SQL for new columns
4. **Estimate effort** - Per phase, for planning

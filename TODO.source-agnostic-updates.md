# Source Agnostic Updates

## Goal

Move from hardcoded SimplyPlural → Targets architecture to a flexible Source → Target model where:
- Any supported platform can be a **source** (SP, PK, manual)
- Any supported platform can be a **target** (VRChat, Discord, PK, Website)
- Users can configure **per-field sourcing** (e.g., fronters from SP, names/avatars from PK)

---

## Architecture Summary

### Fronter Model

```
Fronter (enum)
├── Member { member_id, data: FronterData }
└── CustomFront { custom_front_id, data: FronterData }

FronterData (struct)
├── name, start_time, avatar_url, pluralkit_id, vrchat_status_name, member_created_at, source
```

### Front Structure

```
FrontStructure (enum)
├── Ordered(Vec<Fronter>)       ← PluralKit primary (has explicit ordering)
└── Unordered(HashSet<Fronter>) ← SimplyPlural primary (no ordering)
```

### Source Configuration

```rust
struct SourceConfig {
    primary: PrimarySourceConfig,           // SP, PK, or Manual
    enrichment: Option<EnrichmentSource>,   // Optional secondary source
    field_sourcing: FieldSourcingConfig,    // Per-field: name, avatar, start_time, etc.
}

enum FieldSource {
    Primary,
    Enrichment,
    PrimaryThenEnrichment,
    EnrichmentThenPrimary,
}
```

### Display Configuration

```rust
enum FronterOrdering {
    ByStartTime, ByStartTimeReverse,
    Alphabetical,
    ByMemberCreatedAt, ByMemberCreatedAtReverse,
    SourceOrder,  // Preserve source's native ordering
}
```

---

## Key Design Decisions

| Aspect | Decision |
|--------|----------|
| **Fronter Type** | Enum: Member or CustomFront with shared `FronterData` |
| **Front Structure** | Enum: Ordered (Vec) or Unordered (HashSet) based on source |
| **Multiple Sources** | Primary + single enrichment source |
| **Field Sourcing** | Per-field configuration (name from PK, fronters from SP, etc.) |
| **Fetching** | Parallel fetch with fail-fast (both sources must succeed) |
| **Ordering** | Independent of source - user configured |
| **Custom Fronts** | Users create members for this; no special handling needed |
| **Bridge Compatibility** | No backward compatibility (auto-updating) |
| **Privacy** | Per-source configuration |

---

## Implementation Phases

### Phase 1: Foundation (`base-src`)
- [ ] Create `Fronter` enum (Member/CustomFront with `FronterData`)
- [ ] Create `FrontStructure` enum (Ordered/Unordered)
- [ ] Add `SourcePlatform` enum (SimplyPlural, PluralKit, Manual)
- [ ] Add `FronterOrdering` enum
- [ ] Update `FrontingFormat` to accept `FrontStructure` + ordering

### Phase 2: Source Abstraction
- [ ] Create `Source` trait (`fetch_front()`, `subscribe_changes()`)
- [ ] Implement `SimplyPluralSource` adapter (returns `Unordered`)
- [ ] Implement `PluralKitSource` adapter (returns `Ordered`)
- [ ] Update `change_processor.rs` to use source abstraction

### Phase 3: Enrichment & Merging
- [ ] Implement parallel fetching (fail-fast on any failure)
- [ ] Implement per-field merging logic
- [ ] Add `FieldSourcingConfig` to user config

### Phase 4: Configuration & Database
- [ ] Add new config fields: `primary_source_platform`, `enrichment_source_platform`
- [ ] Add per-field sourcing columns
- [ ] Add `fronter_ordering` column
- [ ] Update config API endpoints

### Phase 5: Frontend Settings UI
- [ ] Source selection (primary + enrichment)
- [ ] Per-field sourcing matrix
- [ ] Ordering configuration

### Phase 6: Bridge Update
- [ ] Update WebSocket protocol to v2 (new `Fronter` format)
- [ ] Update bridge frontend to handle new format
- [ ] Enable auto-update mechanism

### Phase 7: Cleanup
- [ ] Remove legacy SP-specific code paths
- [ ] Deprecate old config fields
- [ ] Update documentation

---

## Open Questions

- [ ] Enrichment conflict resolution (if SP and PK disagree on a name)
- [ ] PK as primary without SP (no custom fronts support)
- [ ] Ordering fallback when source doesn't provide it

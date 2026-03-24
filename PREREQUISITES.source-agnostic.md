# Source-Agnostic Updates - Prerequisites

Before implementing the source-agnostic architecture, the following prerequisites must be completed. These are foundational changes that the main implementation depends on.

---

## Prerequisite 1: Add Pronouns Field to Fronter Model

**Rationale:** Pronouns are a core field that users may want to source from different platforms (e.g., pronouns from PK, names from SP). Currently, the `Fronter` struct has no pronouns field.

### Changes Required

#### 1.1 Update `Fronter` struct in `src/plurality/simply_plural_model.rs`

```rust
#[derive(Clone, Debug)]
pub struct Fronter {
    pub fronter_id: String,
    pub name: String,
    pub pronouns: Option<String>,  // NEW FIELD
    pub avatar_url: String,
    pub pluralkit_id: Option<String>,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub privacy_buckets: Vec<String>,
}
```

#### 1.2 Update `From<Member> for Fronter` implementation

**SimplyPlural member parsing:**
- SP stores pronouns in the member's `info` JSON object
- Field key: typically `"pronouns"` (user-configurable custom field)
- Extract pronouns from member info during deserialization

```rust
impl From<Member> for Fronter {
    fn from(m: Member) -> Self {
        // Extract pronouns from custom fields
        let pronouns = m.content.pronouns_field_id.as_ref().and_then(|field_id| {
            m.content
                .info
                .as_object()
                .and_then(|custom_fields| custom_fields.get(field_id))
                .and_then(|value| value.as_str())
                .map(ToString::to_string)
        });
        
        Self {
            fronter_id: m.member_id,
            name: m.content.name,
            pronouns,  // NEW
            avatar_url: m.content.avatar_url,
            pluralkit_id: m.content.pluralkit_id,
            start_time: None,
            privacy_buckets: m.content.privacy_buckets,
        }
    }
}
```

**Note:** This requires fetching the pronouns field ID from SimplyPlural's custom fields API.

#### 1.3 Update `From<CustomFront> for Fronter` implementation

Custom fronts typically don't have pronouns:
```rust
impl From<CustomFront> for Fronter {
    fn from(cf: CustomFront) -> Self {
        Self {
            fronter_id: cf.custom_front_id,
            name: cf.content.name,
            pronouns: None,  // Custom fronts don't have pronouns
            avatar_url: cf.content.avatar_url,
            pluralkit_id: None,
            start_time: None,
            privacy_buckets: cf.content.privacy_buckets,
        }
    }
}
```

#### 1.4 Update PluralKit API integration

**File:** `src/platforms/to_pluralkit.rs` (and new source adapter)

PluralKit member API returns pronouns:
```json
{
  "id": "member_id",
  "name": "Name",
  "pronouns": "they/them",  // <-- Use this
  "avatar_url": "...",
  ...
}
```

Add pronouns field to PK member parsing.

#### 1.5 Update downstream usage

Search for all `Fronter` usages and update:
- `fronting_status.rs` - formatting logic (optionally include pronouns in display)
- `vrchat.rs` - VRChat status formatting
- `discord.rs` - Discord Rich Presence formatting
- `webview_api.rs` - Website fronting display
- All test files

#### 1.6 Database considerations

**Question:** Should pronouns be stored in history?
- If yes: Update `history` table schema
- If no: No database changes needed

**Recommendation:** Store pronouns in history for accurate historical display.

```sql
-- Optional: Add pronouns to history entries
ALTER TABLE fronting_history ADD COLUMN pronouns JSONB DEFAULT NULL;
-- JSONB stores array of pronouns for each fronter
```

---

## Prerequisite 2: Implement PluralKit Webhook Support

**Rationale:** The source-agnostic architecture assumes all sources can push updates. Currently, PK is polled manually (if at all). Webhooks enable real-time PK fronting changes.

### 2.1 PK Webhook API Overview

PluralKit supports webhooks for system events:
- **Documentation:** https://api.pluralkit.me/v2#webhooks
- **Events:** `switch_created`, `switch_updated`, `switch_deleted`, `member_created`, `member_updated`, `member_deleted`
- **Delivery:** HTTP POST to registered webhook URL
- **Security:** HMAC signature verification

### 2.2 Implementation Components

#### 2.2.1 Webhook Endpoint (Rocket)

**File:** `src/platforms/pluralkit_webhook.rs` (new)

```rust
#[post("/webhook/pluralkit/<user_id>", data = "<payload>")]
async fn handle_pluralkit_webhook(
    user_id: &UserId,
    payload: Json<PluralKitWebhookPayload>,
    db_pool: &State<sqlx::PgPool>,
    fronter_channel: &State<FronterChannel>,
) -> Result<String, Status> {
    // 1. Verify webhook signature
    // 2. Parse event type
    // 3. If switch event, trigger front fetch
    // 4. Send update through fronter channel
}
```

#### 2.2.2 Webhook Payload Model

```rust
#[derive(Deserialize, Debug)]
pub struct PluralKitWebhookPayload {
    pub event: String,  // "switch_created", "switch_updated", etc.
    pub system: PluralKitSystem,
    pub switch: Option<PluralKitSwitch>,
    pub member: Option<PluralKitMember>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize, Debug)]
pub struct PluralKitSystem {
    pub id: String,
    pub uid: String,  // System UID for API calls
}

#[derive(Deserialize, Debug)]
pub struct PluralKitSwitch {
    pub id: String,
    pub members: Vec<String>,  // Member IDs
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```

#### 2.2.3 Webhook Signature Verification

PluralKit signs webhooks with HMAC-SHA256:

```rust
fn verify_webhook_signature(
    payload: &str,
    signature: &str,
    secret: &str,
) -> Result<bool, anyhow::Error> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    
    type HmacSha256 = Hmac<Sha256>;
    
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())?;
    mac.update(payload.as_bytes());
    
    Ok(mac.verify_slice(signature.as_bytes()).is_ok())
}
```

#### 2.2.4 Webhook Registration

**File:** `src/plurality/pluralkit.rs` (new or extend existing)

```rust
pub async fn register_pluralkit_webhook(
    client: &reqwest::Client,
    api_token: &Decrypted,
    webhook_url: String,
    secret: String,
) -> Result<PluralKitWebhook, anyhow::Error> {
    let response = client
        .post("https://api.pluralkit.me/v2/webhooks")
        .header("Authorization", &api_token.secret)
        .json(&PluralKitWebhookRegistration {
            url: webhook_url,
            secret,
            enabled: true,
        })
        .send()
        .await?
        .error_for_status()?
        .json::<PluralKitWebhook>()
        .await?;
    
    Ok(response)
}
```

#### 2.2.5 Webhook Storage

**Database schema:**
```sql
CREATE TABLE pluralkit_webhooks (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    webhook_id VARCHAR(255) NOT NULL,
    webhook_secret VARCHAR(255) NOT NULL,  -- Encrypted
    webhook_url VARCHAR(512) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_triggered_at TIMESTAMPTZ,
    enabled BOOLEAN NOT NULL DEFAULT TRUE
);
```

**Note:** Webhook secret must be encrypted (use existing encryption infrastructure).

#### 2.2.6 Webhook Management API

**Endpoints:**
- `POST /api/pluralkit/webhook/register` - Register webhook with PK
- `DELETE /api/pluralkit/webhook` - Unregister webhook
- `GET /api/pluralkit/webhook/status` - Get webhook status

### 2.3 Fallback: Polling

If webhooks fail or aren't configured, implement polling fallback:

```rust
pub async fn poll_pluralkit_switches(
    config: &UserConfigForUpdater,
    last_known_switch: Option<String>,
) -> Result<Option<PluralKitSwitch>> {
    // Fetch latest switch from PK API
    // Compare with last_known_switch
    // Return Some(new_switch) if different, None otherwise
}
```

**Polling interval:** Configurable (default: 5 minutes)

```sql
ALTER TABLE user_config ADD COLUMN pluralkit_poll_interval_seconds INTEGER DEFAULT 300;
```

### 2.4 Integration with Change Processor

Update `change_processor.rs` to accept PK webhook events:

```rust
// Current: Only SP WebSocket triggers updates
// New: SP WebSocket OR PK Webhook OR polling triggers updates

pub async fn run_listener_for_changes(
    config: users::UserConfigForUpdater,
    // ...
    simply_plural_receiver: LatestReceiver<Vec<Fronter>>,
    pluralkit_receiver: LatestReceiver<Vec<Fronter>>,  // NEW
) -> () {
    // Merge both channels
    // Process updates from either source
}
```

---

## Prerequisite 3: SimplyPlural Pronouns Field Configuration

**Rationale:** SP stores pronouns in custom fields (user-configurable). Need to fetch the pronouns field ID from SimplyPlural's custom fields API.

### 3.1 Fetch Pronouns Field ID

**File:** `src/plurality/simply_plural.rs`

Add function to fetch the pronouns field ID:

```rust
async fn get_pronouns_field_id(
    config: &UserConfigForUpdater,
    system_id: &String,
) -> Result<Option<String>> {
    let custom_fields_url = format!(
        "{}/customFields/{}",
        &config.simply_plural_base_url, system_id
    );
    
    let response = config
        .client
        .get(&custom_fields_url)
        .header("Authorization", &config.simply_plural_token.secret)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let custom_fields: Vec<CustomField> = serde_json::from_str(&response)?;

    // Look for field named "Pronouns" (case-insensitive)
    let pronouns_field = custom_fields
        .iter()
        .find(|field| field.content.name.to_lowercase() == "pronouns");

    Ok(pronouns_field.map(|field| field.id.clone()))
}
```

### 3.2 Store Pronouns Field ID in Config

**Option A:** Auto-detect (recommended)
- Fetch custom fields once during config load
- Cache pronouns field ID in memory
- Re-fetch periodically (e.g., every 24 hours)

**Option B:** User-configured
- Add `pronouns_field_id` to user config
- User manually enters the field ID from SP

**Recommendation:** Option A (auto-detect) for better UX.

### 3.3 Update Member Fetching

In `get_members_and_custom_fronters_by_privacy_rules`:

```rust
let pronouns_field_id = get_pronouns_field_id(config, system_id).await?;

// Pass to member conversion
let mut enriched_member = m;
enriched_member.content.pronouns_field_id = pronouns_field_id.clone();
```

---

## Prerequisite 4: Update Fronting Format to Support Pronouns

**Rationale:** Once pronouns are available, users may want to display them in targets (VRChat status, Discord RPC, website).

### 4.1 Extend `FrontingFormat` Struct

**File:** `src/plurality/fronting_status.rs`

```rust
pub struct FrontingFormat {
    pub max_length: Option<usize>,
    pub cleaning: CleanForPlatform,
    pub prefix: String,
    pub status_if_no_fronters: String,
    pub truncate_names_to_length_if_status_too_long: usize,
    
    // NEW FIELDS
    pub include_pronouns: bool,
    pub pronouns_format: PronounsFormat,  // How to display pronouns
}

pub enum PronounsFormat {
    Parentheses,  // "Annalena (she/her)"
    Slashes,      // "Annalena - she/her"
    Brackets,     // "Annalena [she/her]"
}
```

### 4.2 Update Formatting Logic

```rust
fn collect_clean_fronter_names(
    fronting_format: &FrontingFormat,
    fronts: &[Fronter],
) -> Vec<String> {
    fronts
        .iter()
        .map(|f| {
            let name = match fronting_format.cleaning {
                CleanForPlatform::NoClean => f.name.clone(),
                CleanForPlatform::VRChat => clean_name_for_vrchat_status(&f.name),
            };

            if fronting_format.include_pronouns && f.pronouns.is_some() {
                format!("{} {}", name, format_pronouns(&f.pronouns, &fronting_format.pronouns_format))
            } else {
                name
            }
        })
        .collect()
}
```

### 4.3 Per-Target Configuration

Different targets may have different pronouns display preferences:

```rust
// User config additions
pub enable_pronouns_vrchat: bool,
pub enable_pronouns_discord: bool,
pub enable_pronouns_website: bool,
pub pronouns_format: PronounsFormat,
```

---

## Prerequisite 5: Database Migration Infrastructure

**Rationale:** Source-agnostic updates require new database columns. Set up migration infrastructure first.

### 5.1 Create Migration Files

**Location:** `.sqlx/migrations/`

```bash
.sqlx/
  migrations/
    20260313_add_source_config.sql
    20260313_add_pronouns_field.sql
    20260313_add_pluralkit_webhooks_table.sql
    20260313_add_field_sourcing_config.sql
```

### 5.2 Migration: Source Config

```sql
-- 20260313_add_source_config.sql

-- Primary and enrichment source platforms
ALTER TABLE user_config 
    ADD COLUMN primary_source_platform VARCHAR(50) NOT NULL DEFAULT 'simply_plural',
    ADD COLUMN enrichment_source_platform VARCHAR(50) DEFAULT NULL;

-- Fronter ordering
ALTER TABLE user_config
    ADD COLUMN fronter_ordering VARCHAR(50) NOT NULL DEFAULT 'by_start_time',
    ADD COLUMN fronter_ordering_reverse BOOLEAN NOT NULL DEFAULT FALSE;

-- Validate primary_source_platform values
ALTER TABLE user_config
    ADD CONSTRAINT check_primary_source_platform 
    CHECK (primary_source_platform IN ('simply_plural', 'pluralkit'));

-- Validate enrichment_source_platform values
ALTER TABLE user_config
    ADD CONSTRAINT check_enrichment_source_platform 
    CHECK (enrichment_source_platform IS NULL OR enrichment_source_platform IN ('simply_plural', 'pluralkit'));
```

### 5.3 Migration: Pronouns Field

```sql
-- 20260313_add_pronouns_field.sql

-- Option A: Store in history (recommended)
ALTER TABLE fronting_history 
    ADD COLUMN fronter_data JSONB DEFAULT NULL;
-- JSONB structure: [{fronter_id, name, pronouns, ...}, ...]

-- Option B: Don't store in history (simpler)
-- No changes needed - pronouns computed at display time
```

### 5.4 Migration: PluralKit Webhooks

```sql
-- 20260313_add_pluralkit_webhooks_table.sql

CREATE TABLE pluralkit_webhooks (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    webhook_id VARCHAR(255) NOT NULL,
    webhook_secret TEXT NOT NULL,  -- Encrypted
    webhook_url TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_triggered_at TIMESTAMPTZ,
    enabled BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE INDEX idx_pluralkit_webhooks_enabled ON pluralkit_webhooks(enabled) WHERE enabled = TRUE;
```

### 5.5 Migration: Field Sourcing Config

```sql
-- 20260313_add_field_sourcing_config.sql

ALTER TABLE user_config
    ADD COLUMN name_source VARCHAR(50) NOT NULL DEFAULT 'primary',
    ADD COLUMN pronouns_source VARCHAR(50) NOT NULL DEFAULT 'primary',
    ADD COLUMN start_time_source VARCHAR(50) NOT NULL DEFAULT 'primary',
    ADD COLUMN avatar_source VARCHAR(50) NOT NULL DEFAULT 'primary';

-- Validate field source values
ALTER TABLE user_config
    ADD CONSTRAINT check_field_source
    CHECK (name_source IN ('primary', 'enrichment', 'primary_then_enrichment', 'enrichment_then_primary')),
    ADD CONSTRAINT check_pronouns_source
    CHECK (pronouns_source IN ('primary', 'enrichment', 'primary_then_enrichment', 'enrichment_then_primary')),
    ADD CONSTRAINT check_start_time_source
    CHECK (start_time_source IN ('primary', 'enrichment', 'primary_then_enrichment', 'enrichment_then_primary')),
    ADD CONSTRAINT check_avatar_source
    CHECK (avatar_source IN ('primary', 'enrichment', 'primary_then_enrichment', 'enrichment_then_primary'));
```

### 5.6 Migration: Privacy Config

```sql
-- 20260313_add_privacy_config.sql

ALTER TABLE user_config
    ADD COLUMN respect_member_field_privacy BOOLEAN NOT NULL DEFAULT TRUE;

-- Note: Existing privacy_fine_grained and privacy_fine_grained_buckets remain SP-specific
```

---

## Prerequisite 6: Update TypeScript Bindings

**Rationale:** Frontend needs type-safe access to new backend types.

### 6.1 Add New Types to Specta Export

**File:** `src/bin/ts-bindings.rs`

```rust
// Add exports for:
specta::export::typescript::export_type::<Fronter>(&mut builder)?;
specta::export::typescript::export_type::<FronterData>(&mut builder)?;
specta::export::typescript::export_type::<FronterMetadata>(&mut builder)?;
specta::export::typescript::export_type::<Front>(&mut builder)?;
specta::export::typescript::export_type::<FronterOrdering>(&mut builder)?;
specta::export::typescript::export_type::<SourcePlatform>(&mut builder)?;
specta::export::typescript::export_type::<FieldSourcingConfig>(&mut builder)?;
specta::export::typescript::export_type::<FieldSource>(&mut builder)?;
specta::export::typescript::export_type::<PronounsFormat>(&mut builder)?;
```

### 6.2 Update Existing Type Exports

Update `UserConfigForUpdater` export to include new fields.

### 6.3 Run Binding Generation

```bash
./steps/15-frontend-generate-bindings.sh
```

---

## Prerequisite 7: Metrics Infrastructure

**Rationale:** Per-source metrics needed for monitoring.

### 7.1 Add New Metrics

**File:** `src/metrics.rs` (or create `src/updater/metrics.rs`)

```rust
int_counter_metric!(
    SOURCE_FETCH_REQUESTS_TOTAL,
    "source_fetch_requests_total",
    &["source", "user_id"]
);

int_counter_metric!(
    SOURCE_FETCH_FAILURES_TOTAL,
    "source_fetch_failures_total",
    &["source", "user_id"]
);

int_gauge_metric!(
    SOURCE_FETCH_LATENCY_MS,
    "source_fetch_latency_ms",
    &["source", "user_id"]
);

int_gauge_metric!(
    SOURCE_FRONTERS_COUNT,
    "source_fronters_count",
    &["source", "user_id"]
);
```

### 7.2 Instrument Source Fetching

```rust
// In Source trait implementations
async fn fetch_front(&self, config: &SourceCredentials) -> Result<Vec<Fronter>> {
    let start = std::time::Instant::now();
    SOURCE_FETCH_REQUESTS_TOTAL
        .with_label_values(&[self.platform().as_str(), &config.user_id.to_string()])
        .inc();
    
    let result = self.fetch_front_inner(config).await;
    
    let latency_ms = start.elapsed().as_millis() as i64;
    SOURCE_FETCH_LATENCY_MS
        .with_label_values(&[self.platform().as_str(), &config.user_id.to_string()])
        .set(latency_ms);
    
    if result.is_err() {
        SOURCE_FETCH_FAILURES_TOTAL
            .with_label_values(&[self.platform().as_str(), &config.user_id.to_string()])
            .inc();
    }
    
    result
}
```

---

## Prerequisite Checklist

Before starting main implementation phases, verify:

- [ ] **Pronouns field added to `Fronter` struct**
- [ ] **SP pronouns field ID fetching implemented**
- [ ] **PK member pronouns parsing implemented**
- [ ] **All downstream `Fronter` usages updated**
- [ ] **PK webhook endpoint implemented**
- [ ] **PK webhook signature verification implemented**
- [ ] **PK webhook registration API implemented**
- [ ] **PK webhook storage table created**
- [ ] **PK polling fallback implemented** (optional but recommended)
- [ ] **Database migrations created and tested**
- [ ] **TypeScript bindings generated**
- [ ] **Per-source metrics added**
- [ ] **All tests passing**

---

## Estimated Effort

| Prerequisite | Estimated Time | Dependencies |
|--------------|----------------|--------------|
| 1. Pronouns field | 4-6 hours | None |
| 2. PK webhooks | 12-16 hours | None |
| 3. SP pronouns config | 2-3 hours | Prerequisite 1 |
| 4. Fronting format update | 3-4 hours | Prerequisite 1 |
| 5. Database migrations | 2-3 hours | None |
| 6. TypeScript bindings | 1-2 hours | All above |
| 7. Metrics | 2-3 hours | None |
| **Total** | **26-37 hours** | |

---

## Testing Requirements

Each prerequisite requires:

1. **Unit tests** for new logic
2. **Integration tests** with real APIs (where possible)
3. **Manual testing** for webhook delivery

### PK Webhook Testing

1. Register webhook with PK test system
2. Trigger switch in PK dashboard
3. Verify webhook received and processed
4. Test signature verification (reject invalid signatures)
5. Test polling fallback (disable webhook, verify polling works)

### Pronouns Testing

1. Create SP member with pronouns custom field
2. Verify pronouns fetched correctly
3. Create PK member with pronouns
4. Verify pronouns fetched correctly
5. Test formatting with/without pronouns enabled

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| PK webhook delivery unreliable | High | Implement polling fallback |
| SP pronouns field naming varies | Medium | Auto-detect by name pattern, allow manual override |
| Database migration breaks existing users | High | Test migration on copy of production data |
| Pronouns increase payload size | Low | Monitor payload size, add compression if needed |
| Webhook secret storage security | High | Use existing encryption infrastructure |

---

## Next Steps

1. **Review this document** - Confirm all prerequisites are captured
2. **Prioritize** - Which prerequisites are blocking vs. can be parallelized?
3. **Assign** - Who will implement each prerequisite?
4. **Timeline** - When should prerequisites be complete before main implementation starts?

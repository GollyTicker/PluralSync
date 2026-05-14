# SimplyPlural Deprecation Implementation Plan

**Goal:** Disable SimplyPlural as a source before the backend shuts down July 1, 2026.

**Architecture:** Two hardcoded UTC dates control the deprecation. Backend validates API requests. Cron job disables all existing SP configs. Frontend imports the date via ts-bindings and disables the panel.

---

### Task 1: Add Rust date constants

**Files:** `base-src/src/meta.rs`

Add after `PLURALSYNC_GITHUB_REPOSITORY_RELEASES_URL`:

```rust
use chrono::Datelike;

pub const SIMPLY_PLURAL_DEPRECATION_DATE: chrono::DateTime<chrono::Utc> =
    chrono::NaiveDate::from_ymd_opt(2026, 6, 29).unwrap().and_hms_opt(0, 0, 0).unwrap().and_utc();
pub const SIMPLY_PLURAL_SHUTDOWN_DATE: chrono::DateTime<chrono::Utc> =
    chrono::NaiveDate::from_ymd_opt(2026, 7, 1).unwrap().and_hms_opt(0, 0, 0).unwrap().and_utc();
```

---

### Task 2: Export deprecation date via ts-bindings

**Files:** `src/bin/ts-bindings.rs`

1. Add `SIMPLY_PLURAL_DEPRECATION_DATE` to the `pluralsync_base::meta` import (line 21-24).
2. Add to `defs` array after the `PLURALSYNC_GITHUB_REPOSITORY_RELEASES_URL` format string:

```rust
format!("export const SIMPLY_PLURAL_DEPRECATION_DATE: string = \"{}\"", SIMPLY_PLURAL_DEPRECATION_DATE.format("%Y-%m-%dT00:00:00Z"))
```

3. Run `./steps/15-frontend-generate-bindings.sh` to regenerate `frontend/src/pluralsync.bindings.ts`.

---

### Task 3: API validation with testable helper

**Files:** `src/users/config.rs`

Add a helper function and use it in `create_config_with_strong_constraints`. The helper takes a `DateTime<Utc>` parameter so tests can pass a mock "now".

Add after the existing imports (after line 11):

```rust
use pluralsync_base::meta::SIMPLY_PLURAL_DEPRECATION_DATE;

fn is_simply_plural_deprecated(now: chrono::DateTime<chrono::Utc>) -> bool {
    now >= SIMPLY_PLURAL_DEPRECATION_DATE
}
```

In `create_config_with_strong_constraints`, after the PluralKit mutual exclusion check (after line 513), before the source count check:

```rust
    if enable_from_sp && is_simply_plural_deprecated(chrono::Utc::now()) {
        return Err(anyhow!(
            "SimplyPlural source is no longer available as of June 29, 2026"
        ));
    }
```

In the `tests` module, add a test that exercises both branches by passing mock timestamps:

```rust
    #[test]
    fn test_is_simply_plural_deprecated() {
        // Before deprecation date — not deprecated
        assert!(!is_simply_plural_deprecated(
            chrono::NaiveDate::from_ymd_opt(2026, 6, 28).unwrap().and_hms_opt(0, 0, 0).unwrap().and_utc()
        ));
        // On deprecation date — deprecated
        assert!(is_simply_plural_deprecated(
            chrono::NaiveDate::from_ymd_opt(2026, 6, 29).unwrap().and_hms_opt(0, 0, 0).unwrap().and_utc()
        ));
        // After deprecation date — deprecated
        assert!(is_simply_plural_deprecated(
            chrono::NaiveDate::from_ymd_opt(2026, 7, 1).unwrap().and_hms_opt(0, 0, 0).unwrap().and_utc()
        ));
    }
```

---

### Task 4: Cron job to disable all SP configs

**Files:** `src/database/user_config_queries.rs`, `src/updater/manager.rs`, `src/main.rs`

**4a. Add SQL query** in `src/database/user_config_queries.rs` (after `get_user`, before line 66):

```rust
pub async fn get_user_ids_with_enabled_sp(db_pool: &PgPool) -> Result<Vec<uuid::Uuid>> {
    log::debug!("# | db::get_user_ids_with_enabled_sp");
    let ids: Vec<uuid::Uuid> = sqlx::query_scalar!(
        "SELECT id FROM users WHERE enable_from_sp = true"
    )
    .fetch_all(db_pool)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;
    log::debug!("# | db::get_user_ids_with_enabled_sp | found {} users", ids.len());
    Ok(ids)
}
```

**4b. Add cron job functions** in `src/updater/manager.rs` (after `restart_first_long_living_updater`, before line 570):

```rust
use pluralsync_base::meta::SIMPLY_PLURAL_DEPRECATION_DATE;

pub async fn disable_all_simply_plural_configs(
    db_pool: sqlx::PgPool,
    _: reqwest::Client,
    shared_updaters: UpdaterManager,
    application_user_secrets: database::ApplicationUserSecrets,
    _: setup::SmtpConfig,
) -> Result<()> {
    log::debug!("disable_all_simply_plural_configs");

    if chrono::Utc::now() < SIMPLY_PLURAL_DEPRECATION_DATE {
        return Ok(());
    }

    let sp_user_ids = database::get_user_ids_with_enabled_sp(&db_pool).await?;
    log::info!("disable_all_simply_plural_configs | found {} users with SP enabled", sp_user_ids.len());

    let mut migrated_count = 0u32;
    let mut error_count = 0u32;

    for user_id_inner in sp_user_ids {
        let user_id = users::UserId { inner: user_id_inner };
        match disable_sp_for_user(&user_id, &db_pool, &application_user_secrets, &shared_updaters).await {
            Ok(()) => { migrated_count += 1; }
            Err(e) => { error_count += 1; log::warn!("disable_all_simply_plural_configs | {user_id} failed: {e}"); }
        }
    }

    log::info!("disable_all_simply_plural_configs | done | migrated={} errors={}", migrated_count, error_count);
    Ok(())
}

async fn disable_sp_for_user(
    user_id: &users::UserId,
    db_pool: &sqlx::PgPool,
    application_user_secrets: &database::ApplicationUserSecrets,
    shared_updaters: &UpdaterManager,
) -> Result<()> {
    database::modify_user_secrets(db_pool, user_id, application_user_secrets, |config| {
        config.enable_from_sp = false;
        config.simply_plural_token = None;
    })
    .await?;

    let client = setup::make_client()?;
    let _ = api::restart_updater_for_user(user_id, db_pool, application_user_secrets, &client, shared_updaters)
        .await
        .inspect_err(|e| {
            log::warn!("disable_sp_for_user | {user_id} restart failed (token removed, expected): {e}");
        });

    Ok(())
}
```

**4c. Register cron job** in `src/main.rs` (after line 97, after `pluralkit-webhook-verification`):

```rust
    let () = setup::start_cron_job(
        &app_setup.db_pool,
        &app_setup.client,
        &app_setup.shared_updaters,
        &app_setup.application_user_secrets,
        &app_setup.smtp_config,
        "disable-simply-plural-configs",
        setup::EVERY_5_MINUTES,
        updater::disable_all_simply_plural_configs,
    )
    .await?;
```

---

### Task 5: Frontend deprecation UI

**Files:** `frontend/src/components/SimplyPluralConfigPanel.vue`

**5a. Add imports and computed state:**

```typescript
import { ref, watch, computed, type Ref } from 'vue'
import { SIMPLY_PLURAL_DEPRECATION_DATE } from '@/pluralsync.bindings'

const isSimplyPluralDeprecated = computed(() => {
  const depDate = new Date(SIMPLY_PLURAL_DEPRECATION_DATE)
  const now = new Date()
  const nowUTC = new Date(Date.UTC(now.getFullYear(), now.getMonth(), now.getDate()))
  const depUTC = new Date(Date.UTC(depDate.getFullYear(), depDate.getMonth(), depDate.getDate()))
  return nowUTC >= depUTC
})
```

**5b. Add deprecation message** after `<h2>Simply Plural</h2>`:

```vue
<div v-if="isSimplyPluralDeprecated" class="config-description warning">
  SimplyPlural has been shut down. Your configuration is preserved but no longer functional.
</div>
```

**5c. Disable all inputs** — add `:disabled="isSimplyPluralDeprecated"` to every input/select in the template:
- Line 14: `#enable_from_sp` checkbox
- Line 28: `#simply_plural_token` input
- Lines 60, 70, 83, 90: show-members/custom-fronts checkboxes
- Line 119: privacy fine-grained select
- Line 133: privacy buckets select (add `|| isSimplyPluralDeprecated` to existing disabled expression)

**5d. Guard privacy bucket refresh:**

```typescript
async function refreshPrivacyBuckets() {
  if (isSimplyPluralDeprecated.value) return
  // ... rest unchanged
}
```

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

---

### Task 6: Pre-deprecation announcement email

**Files:** `src/users/announcement_email.rs`

Add a new email function before `get_all_announcement_emails()`:

```rust
#[must_use]
pub fn simply_plural_deprecation_warning() -> AnnouncementEmail {
    AnnouncementEmail {
        email_id: "2026-05-simply_plural_deprecation_warning",
        date: "2026-05-20",
        subject_fn: |_| "PluralSync 🔄 - SimplyPlural Source Shut Down on June 29".to_string(),
        body_fn: |_| {
            "Dear PluralSync Users,\n\
            \n\
            This is a reminder that SimplyPlural (api.apparyllis.com), the source service that many PluralSync users sync from, will be shut down on July 1, 2026.\n\
            \n\
            As a precaution, PluralSync will disable all SimplyPlural-based configurations on June 29, 2026 at 00:00 UTC. After this date:\n\
            \n\
            - Your SimplyPlural token will be removed from our servers\n\
            - SimplyPlural will no longer work as a sync source in PluralSync\n\
            - Your existing configuration will be preserved but non-functional\n\
            \n\
            If you currently use SimplyPlural as your source, you will need to switch to another supported source (PluralKit or WebSocket Source) before June 29.\n\
            Check your PluralSync settings to configure an alternative source.\n\
            \n\
            If you don't use SimplyPlural as a source, this change does not affect you.\n\
            \n\
            Thank you for your attention.\n\
            \n\
            Kinds, PluralSync"
                .to_owned()
        },
    }
}
```

Register it in `get_all_announcement_emails()`:

```rust
pub fn get_all_announcement_emails() -> Vec<AnnouncementEmail> {
    vec![
        email_announcements_activated(),
        smiply_plural_discontinuation_1(),
        pluralkit_as_source(),
        developer_absence_in_june(),
        simply_plural_deprecation_warning(),
        // todo. add announcement about asking for donations
    ]
}
```

---

### Task 7: Post-deactivation confirmation email

**Files:** `src/users/announcement_email.rs`

Add a new email function after `simply_plural_deprecation_warning()`:

```rust
#[must_use]
pub fn simply_plural_deactivated() -> AnnouncementEmail {
    AnnouncementEmail {
        email_id: "2026-06-simply_plural_deactivated",
        date: "2026-06-30",
        subject_fn: |_| "PluralSync 🔄 - SimplyPlural Source Has Been Disabled".to_string(),
        body_fn: |_| {
            "Dear PluralSync Users,\n\
            \n\
            As announced previously, SimplyPlural (api.apparyllis.com) has been shut down. On June 29, 2026, PluralSync automatically disabled all SimplyPlural-based configurations in preparation for this event.\n\
            \n\
            If you used SimplyPlural as a sync source:\n\
            \n\
            - Your SimplyPlural token has been removed from our servers\n\
            - Your configuration is preserved but no longer functional\n\
            - You will see a notice in your SimplyPlural settings panel\n\
            \n\
            To continue syncing, please configure an alternative source (PluralKit or WebSocket Source) in your PluralSync settings.\n\
            \n\
            If you don't use SimplyPlural as a source, no action is needed — this change does not affect you.\n\
            \n\
            Thank you for your understanding.\n\
            \n\
            Kinds, PluralSync"
                .to_owned()
        },
    }
}
```

Register it in `get_all_announcement_emails()`:

```rust
pub fn get_all_announcement_emails() -> Vec<AnnouncementEmail> {
    vec![
        email_announcements_activated(),
        smiply_plural_discontinuation_1(),
        pluralkit_as_source(),
        developer_absence_in_june(),
        simply_plural_deprecation_warning(),
        simply_plural_deactivated(),
        // todo. add announcement about asking for donations
    ]
}
```

**How these work with existing infrastructure:**

The announcement email system (already in `announcement_email.rs` + `main.rs` cron job) handles delivery automatically:
- When deployed, `ensure_announcement_email_definitions` creates entries in the DB
- Any user registered **before** the email's `date` becomes eligible to receive it
- The existing cron job in `main.rs:68` sends pending emails with rate limiting
- No new cron job or DB migration is needed — the existing `send_pending_announcement_emails` handles everything

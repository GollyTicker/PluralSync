# SimplyPlural Removal Design

**Date:** 2026-05-15
**Status:** Approved

## Background

SimplyPlural (api.apparyllis.com) shut down on July 1, 2026. PluralSync disabled all SimplyPlural-based configurations on June 29, 2026 per the deprecation plan. This design covers complete removal of all SimplyPlural-related code from the codebase.

## Scope Decisions

- Remove all SimplyPlural-specific code, config fields, and database columns
- Keep SimplyPlural-related announcement emails as historical records (already sent)
- Keep `pluralkit_as_source()` email body unchanged (historical SP mention preserved)
- Drop all SP-specific database columns and the `privacy_fine_grained_enum` type
- Remove unused `SIMPLY_PLURAL_SHUTDOWN_DATE` constant

## Architecture After Removal

The source selection in `fetch_fronters()` collapses from 3 options (SimplyPlural / PluralKit / WebSocket Source) to 2 (PluralKit / WebSocket Source). The `enable_from_sp` field and all SP-exclusive config fields are removed from both the database and code.

---

## Rust Backend Changes

### Files Deleted

- `src/plurality/simply_plural.rs` — HTTP API client for fetching fronters/members/custom fronts from SimplyPlural
- `src/plurality/simply_plural_model.rs` — Model types for SimplyPlural API responses (`FrontEntry`, `Member`, `CustomFront`, `Friend`, `CustomField`, WebSocket event parsing)
- `src/plurality/simply_plural_websocket.rs` — Auto-reconnecting WebSocket client for SimplyPlural real-time updates

### Files Modified

#### `src/plurality/mod.rs`
Remove 3 module declarations and 3 `pub use` lines for `simply_plural`, `simply_plural_model`, `simply_plural_websocket`.

#### `src/plurality/fronting_status.rs`
Simplify `fetch_fronters()` to check only PluralKit and WebSocket sources. Remove `fetch_fronts_from_simply_plural` import.

#### `src/plurality/model.rs`
Remove 3 `ExclusionReason` variants used only by SimplyPlural:
- `FrontNotificationsDisabled`
- `CustomFrontsDisabled`
- `NotInDisplayedPrivacyBuckets`

Keep `ArchivedMemberHidden`, `NonArchivedMemberHidden` (used by PluralKit), and `MemberPrivacyPrivate` (PluralKit-specific).

#### `src/users/config.rs`
Remove from `UserConfigDbEntries`:
- `enable_from_sp: bool`
- `simply_plural_token: Option<Secret>`
- `respect_front_notifications_disabled: bool`
- `show_custom_fronts: bool`
- `privacy_fine_grained: PrivacyFineGrained`
- `privacy_fine_grained_buckets: Option<Vec<String>>`

Remove from `UserConfigForUpdater`:
- `simply_plural_base_url: String`
- `simply_plural_token: database::Decrypted`
- `respect_front_notifications_disabled: bool`
- `show_custom_fronts: bool`
- `privacy_fine_grained: PrivacyFineGrained`
- `privacy_fine_grained_buckets: Option<Vec<String>>`

Remove from `with_defaults()`, `Default`, `metrics_config_values()`, and `create_config_with_strong_constraints()`.

Remove `is_simply_plural_deprecated()` helper function and its test.

Update all test fixtures to remove SP fields.

#### `src/updater/manager.rs`
Remove:
- `create_simply_plural_websocket_listener_task()` method
- `UPDATER_MANAGER_SIMPLY_PLURAL_WEBSOCKET_RELEVANT_CHANGE_MESSAGE_COUNT` metric
- `disable_all_simply_plural_configs()` cron job function
- `disable_sp_for_user()` helper function

Remove the SP websocket task from the `vec![]` in `start_updater()`.

#### `src/main.rs`
Remove the `disable-simply-plural-configs` cron job registration (lines 99-109).

#### `src/database/user_config_queries.rs`
Remove `get_user_ids_with_enabled_sp()` function.

Remove `enable_from_sp` and `simply_plural_token` from SQL queries in `get_user()`, `get_user_secrets()`, and `set_user_config_secrets()`.

#### `src/bin/ts-bindings.rs`
Remove `SIMPLY_PLURAL_DEPRECATION_DATE` from imports and the `defs` array.
Remove `enable_from_sp` and `simply_plural_token` from the `UserConfigDbEntries` TypeScript type string.

#### `base-src/src/meta.rs`
Remove:
- `SIMPLY_PLURAL_DEPRECATION_DATE` constant
- `SIMPLY_PLURAL_SHUTDOWN_DATE` constant
- `is_simply_plural_deprecated()` function

---

## Frontend Changes

### Files Deleted

- `frontend/src/simply_plural_api.ts` — Axios client and `get_privacy_buckets()` for SimplyPlural API
- `frontend/src/components/SimplyPluralConfigPanel.vue` — SimplyPlural configuration UI panel

### Files Modified

The main settings page that includes `SimplyPluralConfigPanel` must remove the component import and usage.

`frontend/src/pluralsync.bindings.ts` will be regenerated from ts-bindings (removes `SIMPLY_PLURAL_DEPRECATION_DATE`, `enable_from_sp`, `simply_plural_token`).

---

## Database Migration

New migration to drop 6 columns from the `users` table:

```sql
ALTER TABLE users DROP COLUMN enable_from_sp;
ALTER TABLE users DROP COLUMN enc__simply_plural_token;
ALTER TABLE users DROP COLUMN respect_front_notifications_disabled;
ALTER TABLE users DROP COLUMN show_custom_fronts;
ALTER TABLE users DROP COLUMN privacy_fine_grained;
ALTER TABLE users DROP COLUMN privacy_fine_grained_buckets;
DROP TYPE privacy_fine_grained_enum;
```

---

## What Stays

The following SimplyPlural-related code is preserved as historical records:

- All 4 SimplyPlural announcement email functions (`smiply_plural_discontinuation_1`, `simply_plural_deprecation_warning`, `simply_plural_deprecation_warning_typo`, `simply_plural_deactivated`)
- Their registration in `get_all_announcement_emails()`
- `frontend/public/announcements.json` (mirrors the email definitions)
- `pluralkit_as_source()` email body (contains historical SimplyPlural reference, preserved as-is)

---

## Risk Assessment

- **Breaking API change:** The `ExclusionReason` enum changes. Any external consumers parsing this enum will need to handle the removal of 5 variants. Since this is an internal API consumed by the frontend (regenerated bindings), the ts-bindings regeneration handles this automatically.
- **Database migration:** Dropping columns is irreversible. The migration should be run before deploying the code change.
- **PluralKit unaffected:** `ArchivedMemberHidden` and `NonArchivedMemberHidden` are preserved because PluralKit uses them. Only SP-only exclusion variants are removed.

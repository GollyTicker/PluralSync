# SimplyPlural Deprecation Design

**Date:** 2026-05-14
**Status:** Approved

## Background

SimplyPlural (api.apparyllis.com) will shut down on July 1, 2026. PluralSync users who use SimplyPlural as a source need to be migrated before that date.

## Deprecation Dates (UTC, compared at 00:00)

- **Deprecation date:** 2026-06-29 — cron job disables all SimplyPlural configs; frontend shows deprecation message
- **Shutdown date:** 2026-07-01 — SimplyPlural backend goes offline

## Design

### 1. Rust Constants

Add two date constants in `base-src/src/meta.rs`:

```rust
use chrono::Datelike;

pub const SIMPLY_PLURAL_DEPRECATION_DATE: chrono::DateTime<chrono::Utc> =
    chrono::NaiveDate::from_ymd_opt(2026, 6, 29).unwrap().and_hms_opt(0, 0, 0).unwrap().and_utc();
pub const SIMPLY_PLURAL_SHUTDOWN_DATE: chrono::DateTime<chrono::Utc> =
    chrono::NaiveDate::from_ymd_opt(2026, 7, 1).unwrap().and_hms_opt(0, 0, 0).unwrap().and_utc();
```

### 2. TypeScript Bindings Export

Export `SIMPLY_PLURAL_DEPRECATION_DATE` in `src/bin/ts-bindings.rs` using the same pattern as `CANONICAL_PLURALSYNC_BASE_URL`:

```rust
format!("export const SIMPLY_PLURAL_DEPRECATION_DATE: string = \"{}\"", DEPRECATION_DATE)
```

This generates a TypeScript constant in `frontend/src/pluralsync.bindings.ts` that the frontend uses for the date check.

### 3. API Validation

In `src/users/config.rs`, function `create_config_with_strong_constraints`:

- If `enable_from_sp == true` and `now >= SIMPLY_PLURAL_DEPRECATION_DATE`, return an error: "SimplyPlural source is no longer available"

This prevents any new or re-enabled SimplyPlural configuration after the deprecation date.

### 4. Cron Job

New function in `src/updater/manager.rs`, following the existing `restart_first_long_living_updater` pattern:

- Runs hourly
- Checks `now >= SIMPLY_PLURAL_DEPRECATION_DATE`
- Uses a direct SQL query on the user config table to find users where `enable_from_sp = true` (avoids fetching full configs just to check one field)
- For each user:
  - Sets `enable_from_sp = false`
  - Sets `simply_plural_token = None`
  - Restarts updaters for that user
- Skips per-user errors (deleted user, DB failure), continues to next user
- Logs count of migrated users

~180 users at ~1s each = ~3 minutes total.

### 5. Frontend

In `SimplyPluralConfigPanel.vue`:

- Import `SIMPLY_PLURAL_DEPRECATION_DATE` from bindings
- Compare `now >= new Date(deprecation_date)` (00:00 UTC)
- If deprecated: set all inputs `disabled`, show message: "SimplyPlural has been shut down. Your configuration is preserved but no longer functional."

## No Changes To

- Database schema (no migration needed)
- `src/plurality/fronting_status.rs` — cron disables `enable_from_sp` before shutdown, so SP code path is never reached
- `src/bin/pluralsync-global-manager.rs` — uses a separate admin token, independent of user configs
- `src/plurality/simply_plural_websocket.rs` — token removal by cron triggers the empty-token early return
- `frontend/src/simply_plural_api.ts` — panel disable prevents privacy bucket fetch calls
- `src/database/` — no schema or query changes

## Risk Mitigation

- Cron job is idempotent — safe to run multiple times
- Per-user error skipping — one failure doesn't block others
- API validation provides a second enforcement layer alongside the cron job
- Frontend uses the same date constant as the backend (via ts-bindings), ensuring consistency

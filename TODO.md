# Configurable Fronter Channel Wait Increment

## Goal
Make the `wait_increment` parameter in `manager.rs` (lines 273-299, `recreate_fronter_channel`) user-configurable, while `wait_max` and `duration_to_count_over` are calculated automatically.

## Configuration
- **Field name**: `fronter_channel_wait_increment`
- **Unit**: milliseconds (INTEGER)
- **Default**: 100ms
- **Valid range**: 100ms to 180000ms (3 minutes)
- **User-facing name**: "Minimum Sync Delay"
- **Description**: "Whenever switches occur, PluralSync will wait at least this delay before pushing the sync to the other platforms. This can be useful if you want to register multiple switches in a short duration and want to avoid small short-duration switches on the platforms (such as PluralKit)."

## Calculation Formula (Production Only)
Given user-configured `wait_increment`:
- `wait_max` = min(180s, `wait_increment` × 600) → capped at 3 minutes
- `duration_to_count_over` = min(300s, `wait_increment` × 2400) → capped at 5 minutes

### Examples (Production)
| wait_increment | wait_max | duration_to_count_over |
|----------------|----------|------------------------|
| 100ms (default) | 60s | 4min |
| 500ms | 180s (capped) | 300s (capped) |
| 1000ms | 180s (capped) | 300s (capped) |
| 180000ms (max) | 180s (capped) | 300s (capped) |

### Debug Mode
Debug mode uses fixed hardcoded values (unchanged):
- `wait_increment` = 100ms
- `wait_max` = 1s
- `duration_to_count_over` = 5s

## Implementation Tasks

### 1. Database Migration
- [ ] Create `docker/migrations/016_add_fronter_channel_wait_increment.sql`
  - Add `fronter_channel_wait_increment INTEGER` column to `users` table
  - Set default value to 100

### 2. Backend - Config Structure (`src/users/config.rs`)
- [ ] Add `fronter_channel_wait_increment: Option<i32>` to `UserConfigDbEntries`
- [ ] Add `fronter_channel_wait_increment: usize` to `UserConfigForUpdater`
- [ ] Add default value (100) in `impl Default for UserConfigDbEntries`
- [ ] Add to `with_defaults()` method
- [ ] Add validation in `create_config_with_strong_constraints()`:
  - Range check: 100..=180000
- [ ] Add to `metrics_config_values()` if needed

### 3. Backend - Database Queries (`src/database/user_config_queries.rs`)
- [ ] Add to SELECT in `get_user()`
- [ ] Add to SELECT in `get_user_secrets()`
- [ ] Add to UPDATE in `set_user_config_secrets()` (add parameter binding)

### 4. Backend - Manager (`src/updater/manager.rs`)
- [ ] Modify `recreate_fronter_channel(&self, user_id: &UserId)` to accept config parameter
- [ ] In production mode:
  - Read `wait_increment` from config
  - Calculate `wait_max` = min(180s, wait_increment × 600)
  - Calculate `duration_to_count_over` = min(300s, wait_increment × 2400)
- [ ] In debug mode: keep hardcoded values unchanged
- [ ] Pass calculated values to `RateLimitedMostRecentSend::new()`

### 5. TypeScript Bindings (`src/bin/ts-bindings.rs`)
- [ ] Add `fronter_channel_wait_increment?: number;` to exported type

### 6. Frontend
- [ ] Add to `frontend/src/pluralsync.bindings.ts`
- [ ] Add UI control in appropriate config panel (e.g., `WebsiteConfigPanel.vue` or create new panel)
  - Input type: number
  - Min: 100, Max: 180000, Step: 100
  - Display unit: milliseconds or seconds (user-friendly)
  - Add label and help text matching the description above
- [ ] Update e2e tests if applicable

### 7. Testing
- [ ] Add unit test for validation (range check)
- [ ] Add unit test for calculation formula (production mode)
- [ ] Verify debug mode still uses hardcoded values
- [ ] Run existing integration tests to ensure no regressions

## Notes
- In debug mode, the configuration is ignored and fixed values are used
- In production, the default value (100ms) yields the current behavior
- The calculation ensures reasonable scaling while capping at sensible maximums


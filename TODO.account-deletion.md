# Account Deletion Feature

## Requirements
On the Settings page, add a button for account deletion. When clicking it, the user sees a confirmation modal with:
- **Big warning** explaining consequences of deletion
- **Password field** for security verification
- **Text confirmation**: User must type "delete"
- After confirmation, account is deleted and notification email sent

## Technical Design

### Modal Warning Text
The modal must make clear to users what deletion entails:
```
⚠️ WARNING: This action is permanent and cannot be undone.

When you delete your account:
• All PluralSync updaters will stop and be permanently removed
• Your fronters will NO LONGER sync to Discord, VRChat, or other platforms
• Your website fronting page will stop functioning
• All saved data will be deleted (authentication tokens, platform credentials, sync history)
• Your account cannot be recovered

This is irreversible.
```

### Updater System Architecture
- **Location**: `src/updater/manager.rs` - `UpdaterManager` struct
- **Per-user tasks**: 3 concurrent tasks spawned per user:
  1. Work loop - processes fronter changes
  2. Foreign status updater - listens to platform status updates
  3. Simply Plural WebSocket listener - receives updates
- **Stopping mechanism**: `blocking_abort_and_clear_tasks()` aborts tasks gracefully via Tokio abort signals
- **Current usage**: `restart_updater()` stops old tasks before starting new ones

### Refactoring Required
The `restart_updater()` function must be refactored (currently does both stop + start):

**New functions needed in `UpdaterManager` (src/updater/manager.rs):**
1. `stop_updater(user_id)` - Aborts all tasks, clears channels and statuses
2. `start_updater(user_id, config, db_pool, secrets)` - Creates and spawns new tasks
3. Refactor `restart_updater()` to call `stop_updater()` + `start_updater()`

**Benefit**: Account deletion endpoint can call only `stop_updater()` without restarting.

### API Endpoint Design
- **Method**: `DELETE /api/user`
- **Requirements**: 
  - JWT auth (extracts user_id from token)
  - Password confirmation (body: password field)
  - Confirmation string "delete" (body: confirmation field)
- **Actions**:
  1. Extract user_id from JWT token
  2. Validate password against database hash
  3. Validate confirmation string == "delete"
  4. Call `updater_manager.stop_updater(&user_id)` to stop syncs
  5. Delete user from database (cascading deletes related records)
  6. Send deletion notification email
  7. Return 204 No Content
- **Error codes**:
  - 401: Unauthorized (invalid password)
  - 400: Bad Request (missing/invalid confirmation string)
  - 500: Internal Server Error (DB or email failure)

### Frontend Design
- **Settings Button**: Add "Delete Account" button in `Config.vue` (Settings page) in danger zone
- **Deletion Page**: Create new component `DeleteAccount.vue` (e.g., route `/settings/delete-account`)
- **Page content**:
  - Display warning text with consequences
  - Password input field
  - Text input: "Type 'delete' to confirm"
  - "Cancel" button (returns to settings) and "Delete Account" button (disabled until all fields valid)
  - Submit: `DELETE /api/user` with password + confirmation string
  - On success: Logout user, redirect to homepage, show success message
  - On error: Show validation error messages on the page
- **Navigation**: 
  - Button in Config.vue → navigate to `/settings/delete-account`
  - Cancel button → navigate back to `/settings`
  - Success → logout and navigate to `/`

## Implementation Steps

### Phase 1: Backend Refactoring
1. **Refactor `src/updater/manager.rs`**:
   - Extract stop logic into new `stop_updater(&self, user_id: &UserId) -> Result<()>`
   - Extract start logic into new `start_updater(&self, user_id: &UserId, config, db_pool, secrets) -> Result<()>`
   - Update `restart_updater()` to call both
   - No changes to calling code (backward compatible)

### Phase 2: Backend Core Implementation
2. **Add email function in `src/users/email.rs`**:
   - `send_account_deletion_notification(email: &str) -> Result<()>`
   - Email template confirming account deletion with timestamp

3. **Add database deletion in `src/users/mod.rs`** or new deletion module:
   - `delete_user(user_id: &UserId, db_pool: &PgPool) -> Result<()>`
   - Delete from `password_reset_requests` (cascade or explicit)
   - Delete from `users` (cascading deletes related records)
   - Transaction: all-or-nothing

4. **Add API endpoint in `src/users/user_api.rs`**:
   - `DELETE /api/user` handler
   - Validates auth, password, confirmation string
   - Calls `updater_manager.stop_updater(&user_id)?`
   - Calls `delete_user(&user_id, &db)?`
   - Calls `email::send_account_deletion_notification(&user_email)?` (log error but still return 204)
   - Returns 204 No Content

### Phase 3: Frontend Implementation
5. **Add delete button to `frontend/src/components/Config.vue`**:
   - "Delete Account" button in danger zone styling
   - On click: Navigate to `/settings/delete-account`

6. **Create deletion page `frontend/src/components/DeleteAccount.vue`**:
   - Display warning text with consequences
   - Password input field
   - "Type 'delete' to confirm" text input
   - "Cancel" button (navigate back to /settings) and "Delete Account" button (disabled until fields valid)

7. **Implement deletion logic in DeleteAccount.vue**:
   - Submit: DELETE to `/api/user` with body: `{password, confirmation: "delete"}`
   - On success: Clear auth, redirect to homepage, show success message
   - On failure: Show error message on page

### Phase 4: Testing
8. **Unit tests**:
   - Test `delete_user()` function with transactions
   - Test password validation
   - Test confirmation string validation

9. **Integration tests**:
   - End-to-end deletion flow (auth → page → confirm → verify user deleted)
   - Verify email sent
   - Verify updaters stopped before deletion


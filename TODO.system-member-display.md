# TODO

This document outlines the steps to add the feature for system member specific display names.

## Architecture Overview

*   **Backend:** Rust, using the Rocket web framework and SQLx for database access.
*   **Frontend:** Vue.js 3 with Vite and axios for API calls.
*   **Database:** PostgreSQL, with migrations in `docker/migrations`.
*   **Member Data:** Member information is fetched from the Simply Plural API, not stored in the local database. Custom display names will be stored in a new `member_display_names` table.

## Detailed Implementation Steps

### 1. Database Modification

*   **File:** `docker/migrations/012_create_member_display_names.sql` (Create this file)
*   **Action:** Create a new table `member_display_names` to store custom names for members.
*   **SQL:**
    ```sql
    CREATE TABLE member_display_names (
        user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        member_sp_id TEXT NOT NULL,
        display_name TEXT NOT NULL,
        PRIMARY KEY (user_id, member_sp_id)
    );
    ```

### 2. Backend Changes (Rust)

*   **File:** `src/database/queries.rs` (or a new file `src/database/members.rs`)
*   **Action:** Create functions to interact with the new `member_display_names` table.
    *   `get_member_display_names(db_pool: &PgPool, user_id: &UserId) -> Result<HashMap<String, String>>`:
        *   Fetches all custom display names for a given user and returns them as a map of `member_sp_id` to `display_name`.
    *   `set_member_display_name(db_pool: &PgPool, user_id: &UserId, member_sp_id: &str, display_name: &str) -> Result<()>`:
        *   Inserts or updates a custom display name for a member. This will use an `ON CONFLICT` clause to handle both cases.

*   **File:** `src/users/members_api.rs` (Create this file)
*   **Action:** Create two new API endpoints.
    *   **`get_api_members`:**
        *   Route: `#[get("/api/members")]`
        *   Logic:
            1. Get the authenticated user's ID.
            2. Fetch the user's config to get the Simply Plural token.
            3. Call `get_member_display_names` to get the custom display names from the database.
            4. Call the Simply Plural API (`/v1/me` then `/v1/members/:systemId`) to get the list of members.
            5. Merge the member list with the custom display names.
            6. Return the combined list as JSON.
    *   **`put_api_member_display_name`:**
        *   Route: `#[put("/api/members/<member_sp_id>/display_name", data = "<data>")]`
        *   Logic:
            1. Get the authenticated user's ID, the `member_sp_id`, and the new `display_name` from the request body.
            2. Call `set_member_display_name` to save the new display name in the database.

*   **File:** `src/users/mod.rs`
*   **Action:** Make the new API module public. (`pub mod members_api;`)

*   **File:** `src/main.rs`
*   **Action:** Mount the new routes in the `routes!` macro.

### 3. Frontend Changes (Vue.js)

*The frontend changes remain largely the same as the API contract has not changed from the frontend's perspective.*

*   **File:** `frontend/src/pluralsync.bindings.ts`
*   **Action:** Define the `Member` type.
    ```typescript
    export interface Member {
      id: string; // This will be the member_sp_id
      name: string;
      display_name: string;
      // other fields from the Simply Plural member object
    }
    ```

*   **File:** `frontend/src/pluralsync_api.ts`
*   **Action:** Add functions to interact with the new backend endpoints.
    ```typescript
    // In pluralsync_api object
    get_members: async function (): Promise<Member[]> { /* ... */ },
    set_member_display_name: async function (member_id: string, display_name: string): Promise<void> { /* ... */ },
    ```

*   **File:** `frontend/src/router.ts`
*   **Action:** Add a new route for the Members page: `{ path: '/members', component: Members }`.

*   **File:** `frontend/src/App.vue`
*   **Action:** Add a navigation link to the Members page in the nav bar.

*   **File:** `frontend/src/components/Members.vue` (Create this file)
*   **Action:** Create the UI for managing member display names. This UI must handle large member lists efficiently, especially on mobile.
    *   **UI:**
        *   Add a quick search bar at the top to filter members by name or display name.
        *   Display members in a list.
        *   Implement pagination to show at most 50 members per page.
    *   **Functionality:**
        *   Fetch the full list of members on component mount.
        *   The search bar should filter the complete list of members in real-time on the frontend.
        *   The paginated view should be based on the (potentially filtered) list.
        *   For each member, show their name and an input field for their `display_name`.
        *   When a `display_name` is changed, debounce the input and call `set_member_display_name` to save it automatically. This avoids excessive API calls.

### 4. Using Display Names and Triggering Updates

This section describes how to integrate the new display names into the fronting status and how to ensure the updater processes refresh when a name is changed.

*   **A. Using Display Names in Fronting Status**
    *   **File:** `src/updater/sp_updater.rs` (or equivalent file responsible for Simply Plural updates)
    *   **Action:** Modify the fronting status formatting logic to use custom display names.
    *   **Details:**
        1.  At the start of the update cycle for a user, call the existing `get_member_display_names` function to fetch the map of custom names.
        2.  When formatting the status string (e.g., "A, B are fronting"), for each member, check if their `member_sp_id` exists in the fetched map.
        3.  If a custom `display_name` exists, use it in the string. Otherwise, fall back to the member's default `name` from the Simply Plural API.

*   **B. Triggering Updater Refresh on Change**
    * Whenever the frontend sends a request to the backend to update a member display name, then we need to call a `fetch_and_update_fronters` on manager.rs.


# TODO: Password Reset Implementation

This document outlines the steps to implement the password reset functionality.

## Architecture Overview

*   **Backend:** Rust (Rocket framework).
*   **Email:** SMTP via the `lettre` crate.
*   **Database:** PostgreSQL.
*   **Frontend:** Vue.js.

## Detailed Implementation Steps

### 1. Dependencies (DONE)

*   **File:** `Cargo.toml` (backend)
*   **Action:** Add `lettre` for SMTP support and `rand` for token generation.
    ```toml
    lettre = { version = "*", features = ["tokio1", "tokio1-rustls", "rustls-platform-verifier", "sendmail", "dkim"] }
    rand = "*"
    ```

### 2. Database Modification (DONE)

*   **File:** `docker/migrations/013_create_password_resets.sql` (Create this file)
*   **Action:** Create a table to store reset tokens.
*   **SQL:**
    ```sql
    CREATE TABLE password_resets (
        token TEXT PRIMARY KEY,
        user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        expires_at TIMESTAMPTZ NOT NULL
    );
    ```

### 3. Backend Changes (Rust)

*   **File:** `src/setup.rs` (DONE - `SmtpConfig` and its integration are already implemented)
    *   **Action:** Define a new public struct `SmtpConfig` to hold SMTP settings (`host`, `port`, `username`, `password`, `from_email`).
    *   **Action:** Add the SMTP fields to `ApplicationConfig` and populate them from environment variables in `from_env`. Use prefixes like `SMTP_HOST`.
    *   **Action:** Add `pub smtp_config: SmtpConfig` to the `ApplicationSetup` struct.
    *   **Action:** In `application_setup`, create an instance of `SmtpConfig` from `ApplicationConfig` and include it in the returned `ApplicationSetup`.

*   **File:** `src/database/queries.rs` (DONE - `get_user_id` will be used directly, and its `Result` will be handled in `post_api_auth_forgot_password` where an `Err` will be treated as "user not found" for email enumeration prevention.)
    *   **Action:** Add a query to find a user by their email address: `get_user_by_email(...) -> Result<Option<User>>`.
        *   **STATUS:** Existing `database::get_user_id` returns `Result<UserId>` and errors if not found, which conflicts with the requirement for `post_api_auth_forgot_password` to always return `200 OK` (to prevent email enumeration). This needs to be resolved before proceeding with `post_api_auth_forgot_password`.
    *   **Action:** Add queries to manage password reset tokens: (DONE)
        *   `create_reset_token(...)`: Insert a new token with an expiration time (e.g., 1 hour).
        *   `verify_reset_token(...) -> Result<Option<Uuid>>`: Find a valid, non-expired token and return the `user_id`.
        *   `delete_reset_token(...)`: Remove a token after use.
    *   **Action:** Add a query to update a user's password: `update_user_password(...)`. (DONE)

*   **File:** `src/users/email.rs` (DONE - Function `send_reset_email` is implemented here, not `src/utils/email.rs` as originally noted)
*   **File:** `src/users/user_api.rs` (Partial - Imports and request structs are added, endpoints are pending)
*   **Action:** Add two new endpoints.
    *   `post_api_auth_forgot_password` (`POST /api/auth/forgot-password`): (PENDING)
        *   **Arguments:** `db: &State<PgPool>`, `smtp_config: &State<SmtpConfig>`, `Json<...>`
        *   Body: `{ "email": "..." }`
        *   Logic:
            1.  Generate a secure random token using `rand`.
            2.  Look up user by email using `database::get_user_id`. If `Ok(user_id)` is returned, create and store the token in the DB and asynchronously send the reset email. If `Err(...)` is returned, treat it as user not found and proceed as if an email was sent.
            3.  Always return `200 OK` to prevent email enumeration.
    *   `post_api_auth_reset_password` (`POST /api/auth/reset-password`): (PENDING - Requires `get_user_by_email` resolution for token verification)
        *   **Arguments:** `db: &State<PgPool>`, `Json<...>`
        *   Body: `{ "token": "...", "new_password": "..." }`
        *   Logic:
            1.  Verify token is valid and not expired. Return `400 Bad Request` if not.
            2.  Hash the new password using the existing password utility.
            3.  Update the user's password in the `users` table.
            4.  Delete the used token from the `password_resets` table.
            5.  Return `200 OK`.

*   **File:** `src/main.rs` (PENDING - Requires endpoints to be implemented in `src/users/user_api.rs` before registering routes and managing state)
*   **Action:** In `run_webserver`, add the `SmtpConfig` to Rocket's managed state: `.manage(setup.smtp_config)`.
*   **Action:** In `run_webserver`, register the new routes in the `routes!` macro:
    *   `users::auth_api::post_api_auth_forgot_password`
    *   `users::auth_api::post_api_auth_reset_password`

### 4. Frontend Changes (Vue.js) (PENDING)

*   **File:** `frontend/src/pluralsync_api.ts`
*   **Action:** Add API functions `forgotPassword(email)` and `resetPassword(token, newPassword)`.

*   **File:** `frontend/src/views/ForgotPassword.vue` (Create this file)
*   **Action:** Create a simple form asking for the user's email address.

*   **File:** `frontend/src/views/ResetPassword.vue` (Create this file)
*   **Action:** Create a form asking for the new password. The `token` should be extracted from the URL query parameters.

*   **File:** `frontend/src/router.ts`
*   **Action:** Register the new routes `/forgot-password` and `/reset-password`.

## Email Provider Strategy

*   **Implementation:** Implement generic SMTP support using `lettre`.
*   **Development:** Use a Gmail account with an App Password for testing.
*   **Production:** Switch to a transactional email provider (e.g., SendGrid, Mailgun, AWS SES) via environment variables to ensure deliverability.
    * Note: I've created an account at Brevo and I'm currently waiting for the DNS records to propagate so that the emails can be authentically sent.

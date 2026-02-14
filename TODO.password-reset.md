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

*   **File:** `docker/migrations/013_create_password_reset_requests.sql` (Already created)
*   **Action:** Table `password_reset_requests` to store reset tokens is already created.

### 3. Backend Changes (Rust)

*   **File:** `src/setup.rs` (DONE - `SmtpConfig` and its integration are already implemented)
    *   **Action:** Define a new public struct `SmtpConfig` to hold SMTP settings (`host`, `port`, `username`, `password`, `from_email`).
    *   **Action:** Add the SMTP fields to `ApplicationConfig` and populate them from environment variables in `from_env`. Use prefixes like `SMTP_HOST`.
    *   **Action:** Add `pub smtp_config: SmtpConfig` to the `ApplicationSetup` struct.
    *   **Action:** In `application_setup`, create an instance of `SmtpConfig` from `ApplicationConfig` and include it in the returned `ApplicationSetup`.

*   **File:** `src/database/queries.rs` (DONE)
    *   **Action:** Add a query to find a user by their email address: `get_user_by_email(...) -> Result<Option<User>>`.
        *   **STATUS:** The `database::get_user_id` function is currently used. It returns `Result<UserId>` and errors if not found. For `post_api_auth_forgot_password`, this error is caught and handled internally to always return `200 OK` (to prevent email enumeration), thus achieving the desired outcome without a `get_user_by_email` returning `Option<User>`.
    *   **Action:** Add queries to manage password reset tokens: (DONE)
        *   `create_reset_token(...)`: Implemented as `create_password_reset_request(...)`. Inserts a new token hash with an expiration time.
        *   `verify_reset_token(...) -> Result<Option<Uuid>>`: Implemented as `verify_password_reset_request_matches(...)`. Finds a valid, non-expired token hash and returns the `user_id`.
        *   `delete_reset_token(...)`: Implemented as `delete_password_reset_request(...)`. Removes a token after use.
    *   **Action:** Add a query to update a user's password: `update_user_password(...)`. (DONE)

*   **File:** `src/users/email.rs` (DONE - Function `send_reset_email` is implemented here, not `src/utils/email.rs` as originally noted)
*   **File:** `src/users/user_api.rs` (Partial - Request structs are added, endpoints are pending)
*   **Action:** Add two new endpoints.
    *   `post_api_auth_forgot_password` (`POST /api/auth/forgot-password`): (PENDING)
        *   **Arguments:** `db: &State<PgPool>`, `smtp_config: &State<SmtpConfig>`, `Json<ForgotPasswordRequest>`
        *   Body: `{ "email": "..." }`
        *   Logic:
            1.  Generate a secure random token.
            2.  Attempt to get `user_id` by email using `database::get_user_id`.
            3.  If `user_id` is found, hash the token, store the token hash and expiration in `password_reset_requests` table, and asynchronously send the reset email with the unhashed token.
            4.  Always return `200 OK` to prevent email enumeration, regardless of whether the email exists or the email sending was successful.
    *   `post_api_auth_reset_password` (`POST /api/auth/reset-password`): (PENDING)
        *   **Arguments:** `db: &State<PgPool>`, `Json<ResetPasswordRequest>`
        *   Body: `{ "token": "...", "new_password": "..." }`
        *   Logic:
            1.  Hash the provided token.
            2.  Verify the hashed token against `password_reset_requests` table using `database::verify_password_reset_request_matches`. If invalid or expired, return `400 Bad Request`.
            3.  Hash the `new_password`.
            4.  Update the user's password in the `users` table using `database::update_user_password`.
            5.  Delete the used token from the `password_reset_requests` table using `database::delete_password_reset_request`.
            6.  Return `200 OK`.

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

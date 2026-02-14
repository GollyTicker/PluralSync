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
    lettre = { version = "*", default-features = false, features = [
        "builder",
        "hostname",
        "pool",
        "smtp-transport",
        "tokio1",
        "tokio1-rustls",
        "ring",
        "rustls-platform-verifier",
        "smtp-transport",
        "serde",
        "dkim",
    ] }    ```

### 2. Database Modification (DONE)

*   **File:** `docker/migrations/012_add_password_resets.sql` (Already created)
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
*   **File:** `src/users/user_api.rs` (Partial - Request structs are added, endpoints are implemented)
*   **Action:** Add two new endpoints.
    *   `post_api_auth_forgot_password` (`POST /api/auth/forgot-password`): (DONE)
        *   **Arguments:** `db: &State<PgPool>`, `smtp_config: &State<SmtpConfig>`, `Json<ForgotPasswordRequest>`
        *   Body: `{ "email": "..." }`
        *   Logic:
            1.  Generate a secure random token.
            2.  Attempt to get `user_id` by email using `database::get_user_id`.
            3.  If `user_id` is found, hash the token, store the token hash and expiration in `password_reset_requests` table, and asynchronously send the reset email with the unhashed token.
            4.  Always return `200 OK` to prevent email enumeration, regardless of whether the email exists or the email sending was successful.
    *   `post_api_auth_reset_password` (`POST /api/auth/reset-password`): (DONE)
        *   **Arguments:** `db: &State<PgPool>`, `Json<ResetPasswordRequest>`
        *   Body: `{ "token": "...", "new_password": "..." }`
        *   Logic:
            1.  Hash the provided token.
            2.  Verify the hashed token against `password_reset_requests` table using `database::verify_password_reset_request_matches`. If invalid or expired, return `400 Bad Request`.
            3.  Hash the `new_password`.
            4.  Update the user's password in the `users` table using `database::update_user_password`.
            5.  Delete the used token from the `password_reset_requests` table using `database::delete_password_reset_request`.
            6.  Return `200 OK`.

*   **File:** `src/main.rs` (DONE)
*   **Action:** In `run_webserver`, add the `SmtpConfig` to Rocket's managed state: `.manage(setup.smtp_config)`.
*   **Action:** In `run_webserver`, register the new routes in the `routes!` macro:
    *   `users::user_api::post_api_auth_forgot_password`
    *   `users::user_api::post_api_auth_reset_password`

### 4. Frontend Changes (Vue.js) (DONE)

This section outlines the steps to implement the password reset functionality in the Vue.js frontend.

*   **File:** `frontend/src/pluralsync_api.ts`
    *   **Action:** Add a function `forgotPassword(email: string): Promise<void>` that sends a POST request to `/api/auth/forgot-password` with the user's email.
    *   **Action:** Add a function `resetPassword(token: string, newPassword: string): Promise<void>` that sends a POST request to `/api/auth/reset-password` with the reset token and new password.

*   **File:** `frontend/src/components/ForgotPassword.vue` (Create this component)
    *   **Action:** Design a user interface with an input field for the user's email address.
    *   **Action:** Implement form submission logic to call the `pluralsync_api.forgotPassword` function.
    *   **Action:** Display appropriate feedback to the user (e.g., success message, error message).
    *   **Action:** Redirect the user to a confirmation page or login page after successful submission.

*   **File:** `frontend/src/components/ResetPassword.vue` (Create this component)
    *   **Action:** Design a user interface with input fields for the new password and password confirmation.
    *   **Action:** Extract the `token` from the URL query parameters (`$route.query.token`).
    *   **Action:** Implement form submission logic to call the `pluralsync_api.resetPassword` function, passing the extracted token and new password.
    *   **Action:** Handle potential errors, suchs as invalid or expired tokens.
    *   **Action:** Display appropriate feedback to the user (e.g., success message, error message).
    *   **Action:** Redirect the user to the login page after successful password reset.

*   **File:** `frontend/src/router.ts` (Update the main router configuration)
    *   **Action:** Add a new route for `/forgot-password` that maps to the `ForgotPassword` component.
    *   **Action:** Add a new route for `/reset-password` with a dynamic parameter (e.g., `:token` or handled via query params) that maps to the `ResetPassword` component.
    *   **Action:** Ensure proper navigation guards if necessary (e.g., redirecting authenticated users from these pages).

*   **File:** `frontend/src/components/Login.vue` (If applicable, add link to Login component)
    *   **Action:** Add a "Forgot Password?" link that navigates to the `/forgot-password` route.

## Email Provider Strategy

*   **Implementation:** Implement generic SMTP support using `lettre`.
*   **Development:** Use a Gmail account with an App Password for testing.
*   **Production:** Switch to a transactional email provider (e.g., SendGrid, Mailgun, AWS SES) via environment variables to ensure deliverability.
    * Note: I've created an account at Brevo and I'm currently waiting for the DNS records to propagate so that the emails can be authentically sent.

# TODO: Email verification and changing

This document outlines the steps to implement the email verification and email changing functionality.

## 1. Backend Implementation (Rust)

### 1.1 Database Schema Changes (PostgreSQL)

*   **`users` table:**
    *   Add `new_email` (TEXT, NULLABLE) - Stores a pending new email address during an email change process.
    *   Add `email_verification_token_hash` (TEXT, NULLABLE) - Stores the hashed token for email changes.
    *   Add `email_verification_token_expires_at` (TIMESTAMPTZ, NULLABLE) - Stores the expiry for email change tokens.

*   **`temporary_users` table:** (New table for unverified registrations)
    *   `id` (UUID, Primary Key, Default `gen_random_uuid()`)
    *   `email` (TEXT, NOT NULL, UNIQUE) - The unverified email address.
    *   `password_hash` (TEXT, NOT NULL) - Hashed password for the temporary user.
    *   `email_verification_token_hash` (TEXT, NOT NULL) - Hashed token for initial email verification.
    *   `email_verification_token_expires_at` (TIMESTAMPTZ, NOT NULL) - Expiry for the initial verification token (e.g., 1 hour).
    *   `created_at` (TIMESTAMPTZ, NOT NULL, Default `NOW()`)

### 1.2 API Endpoints

All endpoints will be authenticated, requiring a valid JSON Web Token (JWT).

#### 1.2.0 Register User (`POST /api/user/register`)

*   **Functionality:**
    *   **Check for Existing User:** Before creating a temporary user, check if an account with the provided email already exists in the `users` table. If it does, reject the registration attempt with a `409 Conflict` status.
    *   Generates a unique, time-limited verification token for initial email verification.
    *   Creates or updates an entry in the `temporary_users` table with the user's email, hashed password, hashed verification token, and token expiry (1 hour). If an entry for the email already exists, it is overridden.
    *   Sends an email to the user's registered email address with a verification link containing the token.
*   **Response:** `200 OK` (if temporary user created and email sent successfully), `400 Bad Request` (for invalid credentials), `409 Conflict` (if email already registered and verified).

#### 1.2.1 Verify Email (`POST /api/users/email/verify/{token}`)

*   **Functionality:**
    *   Extracts the token from the URL.
    *   Finds the user by comparing the token (after hashing) against `email_verification_token_hash`.
    *   Checks token expiry.
    *   If valid:
        *   If `new_email` is **NOT** set for the user: Performs initial email verification (sets `email_verified` to `true`, clears token fields).
        *   If `new_email` **IS** set for the user: Performs email change confirmation (updates `email` to `new_email`, sets `email_verified` to `true`, clears `new_email` and token fields).
    *   If invalid/expired: Returns an error.
*   **Response:** `200 OK` (success, maybe redirect to frontend success page), `400 Bad Request`, `404 Not Found` (invalid token).

#### 1.2.2 Request Email Change (`POST /api/users/me/email/change`)

*   **Request Body:** `{ "new_email": "new@example.com" }`
*   **Functionality:**
    *   Validates `new_email` (format, uniqueness, not same as current email).
    *   Generates a unique, time-limited change token.
    *   Stores `new_email`, hashed token, and expiry in the `new_email`, `email_verification_token_hash`, and `email_verification_token_expires_at` fields in the `users` table.
    *   Sends an email to the `new_email` address with a confirmation link containing the token.
    *   Sends an *optional* notification email to the *old* email address about the pending change.
    *   **Note:** Re-uses existing token generation, hashing, and expiry mechanisms.
*   **Response:** `200 OK`, `400 Bad Request` (validation errors), `409 Conflict` (email already exists).

### 1.3 Email Service Integration

*   **Library:** `lettre` will be used (already in use).
*   **Configuration:** Existing SMTP server details and sender email address will be used.
*   **Templates:** A simple text template will be created for verification and change emails.
*   **Async Sending:** Email sending will be non-blocking, using the same approach as for password resets.

### 1.4 Token Generation and Management

*   **Generation:** Use cryptographically secure random token generation (re-using existing functionality).
*   **Hashing:** Hash tokens before storing in the database (re-using existing functionality).
*   **Expiry:** Set a reasonable expiry time (e.g., 24 hours) for tokens (re-using existing functionality).
*   **Invalidation:** Tokens should be single-use.

### 1.5 Security Considerations

*   **Token Security:** Ensure tokens are random, long, and hashed.
*   **HTTPS:** All API communication must be over HTTPS.
*   **Input Validation:** Strict validation for email addresses.
*   **Error Messages:** Expose backend error details to the frontend, except for sensitive cases like password reset.

## 2. Frontend Implementation (Vue.js)

### 2.1 User Interface (UI)

*   **User Settings Page:**

    *   "Change Email" button/section.
*   **Change Email Form:**
    *   Input field for new email.
    *   Submit button.
*   **Notifications:**
    *   Success/error messages for email verification/change requests.
    *   Guidance to check email for verification/confirmation links.
*   **Verification/Confirmation Pages:**
    *   Simple pages to display the result of clicking a verification/confirmation link (e.g., "Email Verified Successfully!", "Email Change Confirmed!").
    *   Handle cases where the token is invalid or expired.
*   **Registration Flow:** Account creation is only considered "full" and login is only enabled *after* email verification.

### 2.2 API Integration

*   **Axios:** Use `axios` to interact with the new backend API endpoints.
*   **JWT:** Ensure JWT is included in all authenticated requests.
*   **Loading States:** Use simple text loading indicators, consistent with other requests.

### 2.3 User Experience (UX) Flows

*   **Initial Registration:** Users must verify their email after initial registration to complete account creation and enable login.
*   **Email Change Flow:**
    1.  User initiates email change from settings.
    2.  User receives email at *new* address to confirm.
    3.  Upon clicking link, user is redirected to frontend confirmation page which makes a call to the backend.
    4.  Confirmation or error message displayed.
*   **Existing Users Migration:** For all current users (pre-mandatory verification), the `email_verified` flag will be set to `true`.

## 3. Shared Concerns

### 3.1 Error Handling

*   **Consistent Error Structure:** Backend should return consistent JSON error objects.
*   **Frontend Error Display:** Expose backend error details, with exceptions for sensitive cases like password reset.

### 3.2 Testing

*   Automated tests will be skipped for this implementation; manual testing will be performed.

### 3.3 Deployment Considerations

*   Deployment concerns will be handled during the implementation phase.

# Announcement Email System - Design Draft

**Status:** Design complete, awaiting implementation  
**Created:** 2026-03-08

---

## Overview

Send announcement emails (e.g., welcome messages, feature announcements) to all users with the following properties:

- Each email sent at most once per user
- Failed sends retried after 4 hours
- Lower priority than operational emails (password reset, verification)
- Only sent if ≤80% of daily email quota used (configurable threshold)
- New users only receive emails created after their registration (no historical emails)
- Queue order: random among eligible users
- Text-only emails for now

---

## Database Schema

### Table 1: `announcement_email_definitions`

Stores metadata about each announcement email type.

```sql
CREATE TABLE announcement_email_definitions (
    email_id VARCHAR(255) PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

- `email_id`: Stable string identifier provided by the Rust function (e.g., `"welcome-march-2026"`)
- `created_at`: When the email definition was first deployed
- Auto-populated on application startup

### Table 2: `pending_emails`

Tracks emails that need to be sent or retried. Absence = successfully sent.

```sql
CREATE TABLE pending_emails (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    email_id VARCHAR(255) NOT NULL REFERENCES announcement_email_definitions(email_id) ON DELETE CASCADE,
    last_attempt TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, email_id)
);
```

- Entry exists = email not yet successfully delivered
- Entry absent = email successfully sent
- On failure: INSERT or UPDATE `last_attempt = NOW()`
- On success: DELETE from table
- No index (as per requirements)

---

## Rust Code Structure

### Module: `src/users/announcement_email.rs`

```rust
use crate::database;
use crate::setup::SmtpConfig;
use anyhow::Result;
use pluralsync_base::users::UserInfo;
use sqlx::PgPool;

/// Represents an announcement email definition
pub struct AnnouncementEmail {
    /// Stable, globally unique identifier (e.g., "welcome-march-2026")
    pub email_id: &'static str,
    /// Function to generate subject line from user info
    pub subject_fn: fn(&UserInfo) -> String,
    /// Function to generate email body from user info
    pub body_fn: fn(&UserInfo) -> String,
}

/// Example: Welcome announcement email
pub fn welcome_announcement_march_2026() -> AnnouncementEmail {
    AnnouncementEmail {
        email_id: "welcome-announcement-march-2026",
        subject_fn: |user| format!("Welcome to PluralSync! 🎉"),
        body_fn: |user| format!(
            "Hi there,\n\n\
            Welcome to PluralSync! You registered on {}.\n\n\
            Kinds, PluralSync",
            user.created_at.format("%B %Y")
        ),
    }
}

/// Registry of all announcement emails
/// Add new emails here when deploying
pub fn get_all_announcement_emails() -> Vec<AnnouncementEmail> {
    vec![
        welcome_announcement_march_2026(),
        // Add new announcement emails here
    ]
}

/// Main entry point: send pending announcement emails
/// 
/// # Arguments
/// * `db_pool` - Database connection pool
/// * `smtp_config` - SMTP configuration for sending emails
/// * `rate_limit_threshold` - Fraction of quota reserved for priority emails (e.g., 0.8 = 80%)
/// * `retry_delay_hours` - Hours to wait before retrying failed sends (e.g., 4)
pub async fn send_pending_announcement_emails(
    db_pool: &PgPool,
    smtp_config: &SmtpConfig,
    rate_limit_threshold: f64,
    retry_delay_hours: i64,
) -> Result<()> {
    // 1. Ensure all registered announcement emails exist in DB (auto-migration)
    let emails = get_all_announcement_emails();
    database::ensure_announcement_email_definitions(db_pool, &emails).await?;

    // 2. Get all pending emails (randomized, respecting retry delay)
    //    Fetch all eligible - we'll stop when rate limit hits
    let pending = database::get_all_pending_announcement_emails(
        db_pool,
        retry_delay_hours,
    ).await?;

    // 3. Send each pending email until rate limit threshold reached
    for (user_id, email_id) in pending {
        // Find the email definition
        let email_def = emails.iter()
            .find(|e| e.email_id == email_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown email_id: {}", email_id))?;

        // Fetch user info for templating
        let user_info = database::get_user_info(db_pool, user_id).await?;

        // Check rate limit with threshold (stops when 80% quota used)
        if let Err(e) = database::try_acquire_email_slot_with_threshold(
            db_pool,
            smtp_config.email_rate_limit_per_day,
            rate_limit_threshold,
        ).await {
            log::warn!("Skipping announcement email (rate limit threshold reached): {}", e);
            return Ok(());  // Stop processing, retry next cron run
        }

        // Generate subject and body
        let subject = (email_def.subject_fn)(&user_info);
        let body = (email_def.body_fn)(&user_info);

        // Send email using existing infrastructure
        let to = pluralsync_base::users::Email { inner: user_info.email.inner };
        match crate::users::email::send_email(db_pool, smtp_config, &to, &subject, body).await {
            Ok(()) => {
                // Success: remove from pending
                database::record_announcement_email_success(db_pool, user_id, email_id).await?;
                log::info!("Sent announcement email '{}' to user {}", email_id, user_id);
            }
            Err(e) => {
                // Failure: record attempt for retry after delay
                database::record_announcement_email_failure(db_pool, user_id, email_id).await?;
                log::error!("Failed to send announcement email '{}' to user {}: {}", email_id, user_id, e);
            }
        }
    }

    Ok(())
}
```

### Module: `src/database/announcement_email_queries.rs`

```rust
use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;
use crate::users::announcement_email::AnnouncementEmail;

/// Ensure all registered announcement emails exist in the definitions table
/// Called on application startup (auto-migration)
pub async fn ensure_announcement_email_definitions(
    db_pool: &PgPool,
    emails: &[AnnouncementEmail],
) -> Result<()> {
    for email in emails {
        sqlx::query!(
            "INSERT INTO announcement_email_definitions (email_id) 
             VALUES ($1) 
             ON CONFLICT (email_id) DO NOTHING",
            email.email_id,
        )
        .execute(db_pool)
        .await?;
    }
    Ok(())
}

/// Get all pending announcement emails ready to send
/// 
/// Returns (user_id, email_id) pairs ordered randomly among eligible users:
/// - User created before email definition (no historical emails for new users)
/// - Not yet successfully sent (or failed and retry window passed)
/// - Ordered by RANDOM()
pub async fn get_all_pending_announcement_emails(
    db_pool: &PgPool,
    retry_delay_hours: i64,
) -> Result<Vec<(Uuid, String)>> {
    let rows = sqlx::query!(
        r#"
        SELECT u.id as user_id, ed.email_id
        FROM users u
        CROSS JOIN announcement_email_definitions ed
        LEFT JOIN pending_emails p ON p.user_id = u.id AND p.email_id = ed.email_id
        WHERE 
            -- User existed before email was created (no historical emails)
            u.created_at < ed.created_at
            AND (
                -- Never attempted (first time sending)
                p.user_id IS NULL
                -- OR failed, but retry window has passed
                OR p.last_attempt < NOW() - ($1 || ' hours')::INTERVAL
            )
        ORDER BY RANDOM()
        "#,
        retry_delay_hours.to_string(),
    )
    .fetch_all(db_pool)
    .await?;

    Ok(rows.into_iter().map(|r| (r.user_id, r.email_id)).collect())
}

/// Record a failed send attempt (insert or update last_attempt)
pub async fn record_announcement_email_failure(
    db_pool: &PgPool,
    user_id: Uuid,
    email_id: &str,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO pending_emails (user_id, email_id, last_attempt)
        VALUES ($1, $2, NOW())
        ON CONFLICT (user_id, email_id)
        DO UPDATE SET last_attempt = NOW()
        "#,
        user_id,
        email_id,
    )
    .execute(db_pool)
    .await?;
    Ok(())
}

/// Record successful send (remove from pending)
pub async fn record_announcement_email_success(
    db_pool: &PgPool,
    user_id: Uuid,
    email_id: &str,
) -> Result<()> {
    sqlx::query!(
        "DELETE FROM pending_emails WHERE user_id = $1 AND email_id = $2",
        user_id,
        email_id,
    )
    .execute(db_pool)
    .await?;
    Ok(())
}
```

### Extension: `src/database/email_rate_limit_queries.rs`

```rust
/// Try to acquire an email slot with a configurable threshold
/// 
/// # Arguments
/// * `limit` - Maximum emails per day
/// * `threshold_ratio` - Fraction of limit allowed for mass emails (e.g., 0.8 for 80%)
/// 
/// Returns error if threshold reached, allowing priority emails to still be sent
pub async fn try_acquire_email_slot_with_threshold(
    db_pool: &PgPool,
    limit: u32,
    threshold_ratio: f64,
) -> Result<()> {
    let row = sqlx::query!(
        "INSERT INTO email_rate_limit (id, current_day, count)
         VALUES (1, CURRENT_DATE, 0)
         ON CONFLICT (id) DO UPDATE
         SET current_day = CURRENT_DATE
         WHERE email_rate_limit.id = 1
         RETURNING count, current_day",
    )
    .fetch_one(db_pool)
    .await?;

    if row.current_day != chrono::Utc::now().date_naive() {
        return Err(anyhow::anyhow!("Email rate limit table has unexpected date"));
    }

    let threshold = (limit as f64 * threshold_ratio) as i64;
    if row.count >= threshold {
        return Err(anyhow::anyhow!(
            "Email rate limit threshold exceeded: {} emails sent today, threshold is {} ({}% of {})",
            row.count,
            threshold,
            (threshold_ratio * 100.0) as i64,
            limit
        ));
    }

    // Increment the count
    sqlx::query!(
        "UPDATE email_rate_limit SET count = count + 1 WHERE id = 1",
    )
    .execute(db_pool)
    .await?;

    Ok(())
}
```

---

## Cron Job Registration

Register in `src/main.rs` or the global manager binary:

```rust
// Run announcement email sender every 5 minutes
// Parameters: threshold=0.8 (80%), retry_delay=4 hours
start_cron_job(
    &db_pool,
    &shared_updaters,
    &application_user_secrets,
    "announcement-email-sender",
    "*/5 * * * * *",  // Every 5 minutes
    |db_pool, _, smtp_secrets| async move {
        // Extract smtp_config from smtp_secrets or pass separately
        let smtp_config = /* get from setup */;
        
        crate::users::announcement_email::send_pending_announcement_emails(
            &db_pool,
            &smtp_config,
            0.8,   // 80% threshold (configurable per call)
            4,     // 4 hour retry delay
        ).await
    },
).await?;
```

---

## Adding New Announcement Emails

1. **Create the email function** in `src/users/announcement_email.rs`:

```rust
pub fn feature_update_april_2026() -> AnnouncementEmail {
    AnnouncementEmail {
        email_id: "feature-update-april-2026",  // Stable ID you provide
        subject_fn: |user| format!("New Feature: Multi-Account Support 🚀"),
        body_fn: |user| format!(
            "Hi there,\n\n\
            We're excited to announce multi-account support!...\n\n\
            Kinds, PluralSync"
        ),
    }
}
```

2. **Register in the registry**:

```rust
pub fn get_all_announcement_emails() -> Vec<AnnouncementEmail> {
    vec![
        welcome_announcement_march_2026(),
        feature_update_april_2026(),  // ← Add here
    ]
}
```

3. **Deploy** - auto-insert on startup handles DB registration, cron job automatically starts queuing

---

## UserInfo Available Fields

The `UserInfo` struct provides these fields for email templating:

```rust
pub struct UserInfo {
    pub id: UserId,                              // UUID
    pub email: Email,                            // String (email address)
    pub password_hash: SecretHashString,         // Not useful for emails
    pub created_at: chrono::DateTime<chrono::Utc>, // Registration timestamp
    pub new_email: Option<Email>,                // Pending email change
    pub email_verification_token_hash: Option<String>,  // Internal
    pub email_verification_token_expires_at: Option<chrono::DateTime<chrono::Utc>>, // Internal
}
```

**Useful for personalization:**
- `email.inner` - User's email address
- `created_at` - Registration date (e.g., "You've been with us since March 2026")

**Not available (would require UserConfigDbEntries):**
- `system_name`
- Display name / username

---

## Deployment Workflow

1. Add new announcement email function to `src/users/announcement_email.rs`
2. Add to `get_all_announcement_emails()` registry
3. Deploy code
4. On startup: `ensure_announcement_email_definitions()` auto-inserts new email_id into DB
5. Cron job automatically picks up and queues for all eligible users

---

## Rate Limiting Behavior

- **Priority emails** (password reset, verification): Always sent, ignore threshold
- **Announcement emails**: Only sent if `current_count < limit * threshold_ratio`
- Default threshold: 80% (configurable via function argument)
- On threshold reached: Stop processing, retry next cron run

---

## Retry Behavior

- **First attempt**: Immediately when email becomes eligible
- **On failure**: Record `last_attempt = NOW()`, retry after 4 hours
- **On success**: Delete from `pending_emails`
- **Retry delay**: Constant 4 hours (not exponential)

---

## Queue Order

- `ORDER BY RANDOM()` among all eligible users
- Ensures fair distribution across user base
- No priority ordering

---

## Implementation Checklist

- [ ] Create SQL migration file for `announcement_email_definitions` and `pending_emails` tables
- [ ] Create `src/database/announcement_email_queries.rs` with database operations
- [ ] Update `src/database/mod.rs` to export new module
- [ ] Create `src/users/announcement_email.rs` with email definitions and sending logic
- [ ] Update `src/users/mod.rs` to export new module
- [ ] Add `try_acquire_email_slot_with_threshold()` to `src/database/email_rate_limit_queries.rs`
- [ ] Update `src/users/email.rs` to export `send_email()` as public (if not already)
- [ ] Register cron job in appropriate binary (`src/main.rs` or `src/bin/pluralsync-global-manager.rs`)
- [ ] Add example announcement email (e.g., welcome email)
- [ ] Test with dev mode (`DANGEROUS_LOCAL_DEV_MODE_PRINT_TOKENS_INSTEAD_OF_SEND_EMAIL=true`)

---

## Open Questions / Future Enhancements

- [ ] **Unsubscribe mechanism**: Add for non-essential emails later
- [ ] **HTML emails**: Currently text-only
- [ ] **Additional user fields**: Fetch `UserConfigDbEntries` for `system_name` personalization if needed
- [ ] **Metrics**: Track announcement email success/failure rates
- [ ] **Admin dashboard**: View pending/sent announcement emails (future UI feature)

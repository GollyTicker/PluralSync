use crate::database;
use crate::setup;
use crate::users;
use crate::users::UserId;
use anyhow::Result;
use futures::never::Never;
use pluralsync_base::controlflow::LoopStreamControl;
use sqlx::PgPool;

/// Represents an announcement email definition
pub struct AnnouncementEmail {
    /// Stable, globally unique identifier (e.g., "welcome-announcement-march-2026")
    pub email_id: &'static str,
    pub subject_fn: fn(&database::UserInfo) -> String,
    pub body_fn: fn(&database::UserInfo) -> String,
}

/// Example: Welcome announcement email
#[must_use]
pub fn welcome_announcement_march_2026() -> AnnouncementEmail {
    AnnouncementEmail {
        email_id: "welcome-announcement-march-2026",
        subject_fn: |_user| "Welcome to PluralSync! 🎉".to_string(),
        body_fn: |user| {
            format!(
                "Hi there,\n\n\
                Welcome to PluralSync! You registered on {}.\n\n\
                Kinds, PluralSync",
                user.created_at.format("%B %Y")
            )
        },
    }
}

/// Registry of all announcement emails
/// Add new emails here when deploying
#[must_use]
pub fn get_all_announcement_emails() -> Vec<AnnouncementEmail> {
    vec![welcome_announcement_march_2026()]
}

/// Main entry point: send pending announcement emails
///
/// # Arguments
/// * `db_pool` - Database connection pool
/// * `smtp_config` - SMTP configuration for sending emails
/// * `rate_limit_threshold` - Fraction of quota reserved for priority emails (e.g., 0.8 = 80%)
/// * `retry_delay_hours` - Hours to wait before retrying failed sends (e.g., 4)
#[allow(clippy::needless_continue)]
pub async fn send_pending_announcement_emails(
    db_pool: &PgPool,
    smtp_config: &setup::SmtpConfig,
    rate_limit_threshold: f64,
    retry_delay_hours: i64,
) -> Result<()> {
    log::debug!("# | send_pending_announcement_emails | Starting announcement email sender");

    let emails = get_all_announcement_emails();
    database::ensure_announcement_email_definitions(db_pool, &emails).await?;

    let pending = database::get_pending_announcement_emails(db_pool, retry_delay_hours).await?;

    log::debug!(
        "# | send_pending_announcement_emails | Found {} pending emails",
        pending.len()
    );

    for (user_id, email_id) in pending {
        match send_pending_email(
            db_pool,
            smtp_config,
            rate_limit_threshold,
            &emails,
            user_id.clone(),
            email_id.clone(),
        )
        .await
        {
            Ok(LoopStreamControl::Break) => break,
            Ok(LoopStreamControl::Continue) => continue,
            Err(e) => {
                log::warn!("Failed to send pending email: {user_id} {email_id}. {e}");
                continue;
            }
        }
    }

    Ok(())
}

async fn send_pending_email(
    db_pool: &sqlx::Pool<sqlx::Postgres>,
    smtp_config: &setup::SmtpConfig,
    rate_limit_threshold: f64,
    emails: &[AnnouncementEmail],
    user_id: UserId,
    email_id: String,
) -> Result<LoopStreamControl<Never>, anyhow::Error> {
    let email_def = emails
        .iter()
        .find(|e| e.email_id == email_id)
        .ok_or_else(|| anyhow::anyhow!("Unknown email_id: {email_id}"))?;
    let user_info = database::get_user_info(db_pool, user_id.clone()).await?;
    let subject = (email_def.subject_fn)(&user_info);
    let body = (email_def.body_fn)(&user_info);
    let to = user_info.email;
    match users::email::send_email_with_threshold(
        db_pool,
        smtp_config,
        &to,
        &subject,
        body,
        rate_limit_threshold,
    )
    .await
    {
        Ok(()) => {
            database::record_announcement_email_success(db_pool, &user_id, &email_id).await?;
            log::info!("Sent announcement email '{email_id}' to user {user_id}");
        }
        Err(e) => {
            // Check if this is a rate limit threshold error - if so, stop processing
            if e.to_string()
                .contains("EMAIL_RATE_LIMIT_THRESHOLD_EXCEEDED")
            {
                log::warn!("Skipping announcement email (rate limit threshold reached): {e}");
                return Ok(LoopStreamControl::Break);
            }
            // Other errors: record failure and continue
            database::record_announcement_email_failure(db_pool, &user_id, &email_id).await?;
            log::warn!("Failed to send announcement email '{email_id}' to user {user_id}: {e}");
        }
    }
    Ok(LoopStreamControl::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Test Constants ===
    const TEST_EMAIL_ID: &str = "welcome-announcement-march-2026";
    const DEFAULT_RETRY_DELAY_HOURS: i64 = 4;
    const DEFAULT_RATE_THRESHOLD: f64 = 0.8;
    const FULL_RATE_THRESHOLD: f64 = 1.0;
    const TEST_PASSWORD_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$test$test";
    const EMAIL_DEFINITION_OFFSET_HOURS: i64 = 1;

    // === Helper Functions ===

    /// Create a base test SMTP config (dev mode, no actual sending)
    fn test_smtp_config() -> setup::SmtpConfig {
        setup::SmtpConfig {
            email_rate_limit_per_day: 100,
            dangerous_local_dev_mode_print_tokens_instead_of_send_email: true,
            host: "smtp.test.com".to_string(),
            port: 587,
            username: "test".to_string(),
            password: "test".to_string(),
            from_email: "test@test.com".to_string(),
            frontend_base_url: "http://test.local".to_string(),
        }
    }

    /// Create a rate-limited SMTP config for testing rate limit behavior
    fn rate_limited_smtp_config(limit: u32) -> setup::SmtpConfig {
        setup::SmtpConfig {
            email_rate_limit_per_day: limit,
            ..test_smtp_config()
        }
    }

    /// Create a test user in the database
    async fn create_test_user(pool: &PgPool, email: &str) -> Result<UserId, anyhow::Error> {
        let user_id = sqlx::query_scalar::<_, uuid::Uuid>(
            "INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING id",
        )
        .bind(email)
        .bind(TEST_PASSWORD_HASH)
        .fetch_one(pool)
        .await?;

        Ok(UserId { inner: user_id })
    }

    /// Get user's created_at timestamp
    async fn get_user_created_at(
        pool: &PgPool,
        user_id: &UserId,
    ) -> Result<chrono::DateTime<chrono::Utc>, sqlx::Error> {
        sqlx::query_scalar("SELECT created_at FROM users WHERE id = $1")
            .bind(user_id.inner)
            .fetch_one(pool)
            .await
    }

    /// Set up an announcement email definition that makes users eligible to receive it
    async fn setup_announcement_email_for_users(
        pool: &PgPool,
        user_created_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), anyhow::Error> {
        // Create email definition with created_at AFTER the users registered
        // This makes the users eligible to receive the email
        // (users registered BEFORE the email was created should receive it)
        sqlx::query(
            "INSERT INTO announcement_email_definitions (email_id, created_at) VALUES ($1, $2)",
        )
        .bind(TEST_EMAIL_ID)
        .bind(user_created_at + chrono::Duration::hours(EMAIL_DEFINITION_OFFSET_HOURS))
        .execute(pool)
        .await?;

        // Create pending entries for eligible users
        let emails = get_all_announcement_emails();
        database::ensure_announcement_email_definitions(pool, emails.as_slice()).await?;
        Ok(())
    }

    /// Assert that a pending email entry exists
    async fn assert_pending_email_exists(
        pool: &PgPool,
        user_id: &UserId,
        email_id: &str,
        message: &str,
    ) {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pending_emails WHERE user_id = $1 AND email_id = $2)",
        )
        .bind(user_id.inner)
        .bind(email_id)
        .fetch_one(pool)
        .await
        .expect("Failed to query pending email existence");
        assert!(exists, "{message}");
    }

    /// Assert that a pending email entry does not exist
    async fn assert_pending_email_not_exists(
        pool: &PgPool,
        user_id: &UserId,
        email_id: &str,
        message: &str,
    ) {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pending_emails WHERE user_id = $1 AND email_id = $2)",
        )
        .bind(user_id.inner)
        .bind(email_id)
        .fetch_one(pool)
        .await
        .expect("Failed to query pending email existence");
        assert!(!exists, "{message}");
    }

    /// Count remaining pending emails for a given email_id
    async fn count_pending_emails(pool: &PgPool, email_id: &str) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM pending_emails WHERE email_id = $1")
            .bind(email_id)
            .fetch_one(pool)
            .await
    }

    // === Tests ===

    #[test]
    fn test_get_all_announcement_emails_returns_non_empty_vec() {
        // === Arrange ===
        // No setup needed

        // === Act ===
        let emails = get_all_announcement_emails();

        // === Assert ===
        assert!(
            !emails.is_empty(),
            "Should return at least one announcement email"
        );
        assert_eq!(
            emails[0].email_id, TEST_EMAIL_ID,
            "First email should be welcome announcement"
        );
    }

    #[sqlx::test(migrations = "docker/migrations")]
    async fn test_send_pending_announcement_emails_empty_queue(
        pool: PgPool,
    ) -> Result<(), anyhow::Error> {
        // === Arrange ===
        let smtp_config = test_smtp_config();

        // === Act ===
        let result = send_pending_announcement_emails(
            &pool,
            &smtp_config,
            DEFAULT_RATE_THRESHOLD,
            DEFAULT_RETRY_DELAY_HOURS,
        )
        .await;

        // === Assert ===
        assert!(result.is_ok(), "Should succeed with empty queue");

        Ok(())
    }

    #[sqlx::test(migrations = "docker/migrations")]
    async fn test_send_pending_announcement_emails_rate_limit_stops_sending(
        pool: PgPool,
    ) -> Result<(), anyhow::Error> {
        // === Arrange ===
        // Reset the rate limit table for this test
        sqlx::query!("DELETE FROM email_rate_limit WHERE id = 1")
            .execute(&pool)
            .await?;

        // Create test users
        let user1_id = create_test_user(&pool, "user1@test.com").await?;
        let _user2_id = create_test_user(&pool, "user2@test.com").await?;
        let _user3_id = create_test_user(&pool, "user3@test.com").await?;

        // Get user created_at and set up announcement email
        let user_created_at = get_user_created_at(&pool, &user1_id).await?;
        setup_announcement_email_for_users(&pool, user_created_at).await?;

        // Set a very low rate limit (2 emails)
        let smtp_config = rate_limited_smtp_config(2);

        // === Act ===
        send_pending_announcement_emails(
            &pool,
            &smtp_config,
            FULL_RATE_THRESHOLD,
            DEFAULT_RETRY_DELAY_HOURS,
        )
        .await?;

        // === Assert ===
        let remaining_pending = count_pending_emails(&pool, TEST_EMAIL_ID).await?;

        // Should have sent 2 emails (rate limit) and stopped
        // So 1 should remain pending (3 users - 2 sent = 1 remaining)
        assert_eq!(
            remaining_pending, 1,
            "Should stop sending after rate limit is reached"
        );

        Ok(())
    }

    #[sqlx::test(migrations = "docker/migrations")]
    async fn test_send_pending_announcement_emails_success_removes_from_pending(
        pool: PgPool,
    ) -> Result<(), anyhow::Error> {
        // === Arrange ===
        // Reset the rate limit table for this test
        sqlx::query!("DELETE FROM email_rate_limit WHERE id = 1")
            .execute(&pool)
            .await?;

        // Create test user
        let user_id = create_test_user(&pool, "success-test@test.com").await?;

        // Get user created_at and set up announcement email
        let user_created_at = get_user_created_at(&pool, &user_id).await?;
        setup_announcement_email_for_users(&pool, user_created_at).await?;

        // Verify it exists before sending
        assert_pending_email_exists(
            &pool,
            &user_id,
            TEST_EMAIL_ID,
            "Pending email should exist before sending",
        )
        .await;

        let smtp_config = test_smtp_config();

        // === Act ===
        send_pending_announcement_emails(
            &pool,
            &smtp_config,
            FULL_RATE_THRESHOLD,
            DEFAULT_RETRY_DELAY_HOURS,
        )
        .await?;

        // === Assert ===
        assert_pending_email_not_exists(
            &pool,
            &user_id,
            TEST_EMAIL_ID,
            "Pending email should be removed after successful send",
        )
        .await;

        Ok(())
    }
}

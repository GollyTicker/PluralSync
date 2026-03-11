use anyhow::Result;
use sqlx::PgPool;

use crate::users::UserId;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::users::announcement_email::AnnouncementEmail;

    #[sqlx::test(migrations = "docker/migrations")]
    async fn test_ensure_announcement_email_definitions_idempotency(pool: PgPool) -> Result<()> {
        let emails = vec![AnnouncementEmail {
            email_id: "test-email-idempotent",
            date: "2026-01-01",
            subject_fn: |_| "Test Subject".to_string(),
            body_fn: |_| "Test Body".to_string(),
        }];

        // First call should succeed
        ensure_announcement_email_definitions(&pool, &emails).await?;

        // Second call should also succeed (idempotent)
        ensure_announcement_email_definitions(&pool, &emails).await?;

        // Verify the email definition exists exactly once
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM announcement_email_definitions WHERE email_id = $1",
        )
        .bind("test-email-idempotent")
        .fetch_one(&pool)
        .await?;

        assert_eq!(count, 1, "Email definition should exist exactly once");

        Ok(())
    }

    #[sqlx::test(migrations = "docker/migrations")]
    async fn test_ensure_announcement_email_definitions_user_eligibility(
        pool: PgPool,
    ) -> Result<()> {
        // Create a test user first
        let user_id = sqlx::query_scalar::<_, uuid::Uuid>(
            "INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING id",
        )
        .bind("test-eligibility@example.com")
        .bind("$argon2id$v=19$m=19456,t=2,p=1$test$test")
        .fetch_one(&pool)
        .await?;

        // Get the user's created_at timestamp
        let user_created_at: chrono::DateTime<chrono::Utc> =
            sqlx::query_scalar("SELECT created_at FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await?;

        // Create an email definition with created_at BEFORE the user registered
        // We need to manually insert with a past timestamp
        sqlx::query(
            "INSERT INTO announcement_email_definitions (email_id, created_at) VALUES ($1, $2)",
        )
        .bind("test-email-before-user")
        .bind(user_created_at - chrono::Duration::hours(1))
        .execute(&pool)
        .await?;

        // Create another email with created_at AFTER the user registered
        sqlx::query(
            "INSERT INTO announcement_email_definitions (email_id, created_at) VALUES ($1, $2)",
        )
        .bind("test-email-after-user")
        .bind(user_created_at + chrono::Duration::hours(1))
        .execute(&pool)
        .await?;

        // Now call ensure for both emails
        let emails = vec![
            AnnouncementEmail {
                email_id: "test-email-before-user",
                date: "2026-01-01",
                subject_fn: |_| "Test".to_string(),
                body_fn: |_| "Test".to_string(),
            },
            AnnouncementEmail {
                email_id: "test-email-after-user",
                date: "2026-01-02",
                subject_fn: |_| "Test".to_string(),
                body_fn: |_| "Test".to_string(),
            },
        ];
        ensure_announcement_email_definitions(&pool, &emails).await?;

        // Check pending_emails:
        // - test-email-before-user: email created BEFORE user → user should NOT receive it
        // - test-email-after-user: email created AFTER user → user SHOULD receive it
        let has_before: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pending_emails WHERE user_id = $1 AND email_id = $2)",
        )
        .bind(user_id)
        .bind("test-email-before-user")
        .fetch_one(&pool)
        .await?;

        let has_after: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pending_emails WHERE user_id = $1 AND email_id = $2)",
        )
        .bind(user_id)
        .bind("test-email-after-user")
        .fetch_one(&pool)
        .await?;

        assert!(
            !has_before,
            "User should NOT have pending email created before their registration"
        );
        assert!(
            has_after,
            "User should have pending email created after their registration"
        );

        Ok(())
    }

    #[sqlx::test(migrations = "docker/migrations")]
    async fn test_ensure_announcement_email_definitions_last_attempt_initialization(
        pool: PgPool,
    ) -> Result<()> {
        // Create a test user first
        let user_id = sqlx::query_scalar::<_, uuid::Uuid>(
            "INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING id",
        )
        .bind("test-last-attempt@example.com")
        .bind("$argon2id$v=19$m=19456,t=2,p=1$test$test")
        .fetch_one(&pool)
        .await?;

        // Get the user's created_at timestamp
        let user_created_at: chrono::DateTime<chrono::Utc> =
            sqlx::query_scalar("SELECT created_at FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await?;

        // Insert email definition AFTER the user was created (so user is eligible)
        sqlx::query(
            "INSERT INTO announcement_email_definitions (email_id, created_at) VALUES ($1, $2)",
        )
        .bind("test-email-last-attempt")
        .bind(user_created_at + chrono::Duration::minutes(1))
        .execute(&pool)
        .await?;

        let emails = vec![AnnouncementEmail {
            email_id: "test-email-last-attempt",
            date: "2026-01-01",
            subject_fn: |_| "Test".to_string(),
            body_fn: |_| "Test".to_string(),
        }];
        ensure_announcement_email_definitions(&pool, &emails).await?;

        // Check that last_attempt is set to a distant past (NOW() - 1 year)
        let last_attempt: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
            "SELECT last_attempt FROM pending_emails WHERE user_id = $1 AND email_id = $2",
        )
        .bind(user_id)
        .bind("test-email-last-attempt")
        .fetch_one(&pool)
        .await?;

        let now = chrono::Utc::now();
        let one_year_ago = now - chrono::Duration::days(365);

        assert!(
            last_attempt < now - chrono::Duration::days(300),
            "last_attempt should be set to a distant past (approximately 1 year ago)"
        );
        assert!(
            last_attempt > one_year_ago - chrono::Duration::days(10),
            "last_attempt should be approximately 1 year ago"
        );

        Ok(())
    }

    #[sqlx::test(migrations = "docker/migrations")]
    async fn test_get_pending_announcement_emails_retry_delay_filter(pool: PgPool) -> Result<()> {
        // Create a test user
        let user_id = sqlx::query_scalar::<_, uuid::Uuid>(
            "INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING id",
        )
        .bind("test-retry-delay@example.com")
        .bind("$argon2id$v=19$m=19456,t=2,p=1$test$test")
        .fetch_one(&pool)
        .await?;

        // Insert email definition
        sqlx::query(
            "INSERT INTO announcement_email_definitions (email_id, created_at) VALUES ($1, $2)",
        )
        .bind("test-email-retry")
        .bind(chrono::Utc::now() - chrono::Duration::days(2))
        .execute(&pool)
        .await?;

        // Insert pending email with last_attempt = 2 hours ago (within 4 hour retry window)
        sqlx::query(
            "INSERT INTO pending_emails (user_id, email_id, last_attempt) VALUES ($1, $2, $3)",
        )
        .bind(user_id)
        .bind("test-email-retry")
        .bind(chrono::Utc::now() - chrono::Duration::hours(2))
        .execute(&pool)
        .await?;

        // Should NOT be returned (within retry window)
        let pending = get_pending_announcement_emails(&pool, 4).await?;
        assert!(
            pending.is_empty(),
            "Email within retry window should not be returned"
        );

        // Update last_attempt to 6 hours ago (outside 4 hour retry window)
        sqlx::query(
            "UPDATE pending_emails SET last_attempt = $1 WHERE user_id = $2 AND email_id = $3",
        )
        .bind(chrono::Utc::now() - chrono::Duration::hours(6))
        .bind(user_id)
        .bind("test-email-retry")
        .execute(&pool)
        .await?;

        // Should be returned (outside retry window)
        let pending = get_pending_announcement_emails(&pool, 4).await?;
        assert_eq!(
            pending.len(),
            1,
            "Email outside retry window should be returned"
        );
        assert_eq!(pending[0].1, "test-email-retry");

        Ok(())
    }

    #[sqlx::test(migrations = "docker/migrations")]
    async fn test_get_pending_announcement_emails_empty_result(pool: PgPool) -> Result<()> {
        let pending = get_pending_announcement_emails(&pool, 4).await?;
        assert!(
            pending.is_empty(),
            "Should return empty vec when nothing is pending"
        );
        Ok(())
    }

    #[sqlx::test(migrations = "docker/migrations")]
    async fn test_record_announcement_email_success_removes_entry(pool: PgPool) -> Result<()> {
        // Create a test user
        let user_id = sqlx::query_scalar::<_, uuid::Uuid>(
            "INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING id",
        )
        .bind("test-success@example.com")
        .bind("$argon2id$v=19$m=19456,t=2,p=1$test$test")
        .fetch_one(&pool)
        .await?;

        // Insert email definition
        sqlx::query(
            "INSERT INTO announcement_email_definitions (email_id, created_at) VALUES ($1, $2)",
        )
        .bind("test-email-success")
        .bind(chrono::Utc::now() - chrono::Duration::days(2))
        .execute(&pool)
        .await?;

        // Insert pending email
        sqlx::query(
            "INSERT INTO pending_emails (user_id, email_id, last_attempt) VALUES ($1, $2, $3)",
        )
        .bind(user_id)
        .bind("test-email-success")
        .bind(chrono::Utc::now() - chrono::Duration::hours(6))
        .execute(&pool)
        .await?;

        // Verify it exists
        let exists_before: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pending_emails WHERE user_id = $1 AND email_id = $2)",
        )
        .bind(user_id)
        .bind("test-email-success")
        .fetch_one(&pool)
        .await?;
        assert!(exists_before, "Entry should exist before recording success");

        // Record success
        record_announcement_email_success(&pool, &UserId { inner: user_id }, "test-email-success")
            .await?;

        // Verify it's removed
        let exists_after: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pending_emails WHERE user_id = $1 AND email_id = $2)",
        )
        .bind(user_id)
        .bind("test-email-success")
        .fetch_one(&pool)
        .await?;
        assert!(
            !exists_after,
            "Entry should be removed after recording success"
        );

        Ok(())
    }

    #[sqlx::test(migrations = "docker/migrations")]
    async fn test_record_announcement_email_failure_updates_timestamp(pool: PgPool) -> Result<()> {
        // Create a test user
        let user_id = sqlx::query_scalar::<_, uuid::Uuid>(
            "INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING id",
        )
        .bind("test-failure@example.com")
        .bind("$argon2id$v=19$m=19456,t=2,p=1$test$test")
        .fetch_one(&pool)
        .await?;

        // Insert email definition
        sqlx::query(
            "INSERT INTO announcement_email_definitions (email_id, created_at) VALUES ($1, $2)",
        )
        .bind("test-email-failure")
        .bind(chrono::Utc::now() - chrono::Duration::days(2))
        .execute(&pool)
        .await?;

        // Insert pending email with old timestamp
        let old_timestamp = chrono::Utc::now() - chrono::Duration::hours(10);
        sqlx::query(
            "INSERT INTO pending_emails (user_id, email_id, last_attempt) VALUES ($1, $2, $3)",
        )
        .bind(user_id)
        .bind("test-email-failure")
        .bind(old_timestamp)
        .execute(&pool)
        .await?;

        // Record failure
        record_announcement_email_failure(&pool, &UserId { inner: user_id }, "test-email-failure")
            .await?;

        // Verify timestamp was updated
        let new_timestamp: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
            "SELECT last_attempt FROM pending_emails WHERE user_id = $1 AND email_id = $2",
        )
        .bind(user_id)
        .bind("test-email-failure")
        .fetch_one(&pool)
        .await?;

        assert!(
            new_timestamp > old_timestamp + chrono::Duration::hours(5),
            "last_attempt should be updated to recent time"
        );

        Ok(())
    }
}

/// Ensure all registered announcement emails exist in the definitions table
/// AND create pending email entries for all eligible users
/// Called on application startup (auto-migration)
pub async fn ensure_announcement_email_definitions(
    db_pool: &PgPool,
    emails: &[crate::users::announcement_email::AnnouncementEmail],
) -> Result<()> {
    for email in emails {
        // Insert the email definition
        sqlx::query!(
            "INSERT INTO announcement_email_definitions (email_id)
             VALUES ($1)
             ON CONFLICT (email_id) DO NOTHING",
            email.email_id,
        )
        .execute(db_pool)
        .await?;

        // Create pending entries for all users who registered BEFORE this email was created
        // Users who registered AFTER the email was created will NOT receive this email
        // (they only receive new emails created after their registration)
        // Set last_attempt to a distant past to make emails immediately eligible for sending
        let email_id = email.email_id.to_string();
        sqlx::query!(
            r#"
            INSERT INTO pending_emails (user_id, email_id, last_attempt)
            SELECT u.id, email_defs.email_id, NOW() - INTERVAL '1 year'
            FROM users u
            CROSS JOIN (SELECT $1::VARCHAR AS email_id) AS email_defs
            WHERE u.created_at < (
                SELECT created_at FROM announcement_email_definitions WHERE email_id = email_defs.email_id
            )
            ON CONFLICT (user_id, email_id) DO NOTHING
            "#,
            email_id,
        )
        .execute(db_pool)
        .await?;
    }
    Ok(())
}

/// Get all pending announcement emails ready to send
///
/// Returns (`user_id`, `email_id`) pairs ordered randomly among eligible users:
/// - Entry exists in `pending_emails` (not yet successfully sent)
/// - Retry window has passed (`last_attempt` + `retry_delay` < `NOW()`)
/// - Ordered by `RANDOM()`
pub async fn get_pending_announcement_emails(
    db_pool: &PgPool,
    retry_delay_hours: i64,
) -> Result<Vec<(UserId, String)>> {
    let rows = sqlx::query!(
        r#"
        SELECT p.user_id, p.email_id
        FROM pending_emails p
        WHERE p.last_attempt < NOW() - ($1 || ' hours')::INTERVAL
        ORDER BY RANDOM()
        "#,
        retry_delay_hours.to_string(),
    )
    .fetch_all(db_pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| (UserId { inner: r.user_id }, r.email_id))
        .collect())
}

/// Record a failed send attempt (update `last_attempt`)
pub async fn record_announcement_email_failure(
    db_pool: &PgPool,
    user_id: &UserId,
    email_id: &str,
) -> Result<()> {
    sqlx::query!(
        "UPDATE pending_emails SET last_attempt = NOW() WHERE user_id = $1 AND email_id = $2",
        user_id.inner,
        email_id,
    )
    .execute(db_pool)
    .await?;
    Ok(())
}

/// Record successful send (remove from pending)
pub async fn record_announcement_email_success(
    db_pool: &PgPool,
    user_id: &UserId,
    email_id: &str,
) -> Result<()> {
    sqlx::query!(
        "DELETE FROM pending_emails WHERE user_id = $1 AND email_id = $2",
        user_id.inner,
        email_id,
    )
    .execute(db_pool)
    .await?;
    Ok(())
}

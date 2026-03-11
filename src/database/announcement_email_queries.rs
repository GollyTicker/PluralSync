use anyhow::Result;
use sqlx::PgPool;

use crate::users::UserId;

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

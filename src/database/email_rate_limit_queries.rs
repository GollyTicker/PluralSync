use anyhow::{Result, anyhow};
use sqlx::PgPool;

/// Try to acquire an email slot with an optional threshold
///
/// # Arguments
/// * `db_pool` - Database connection pool
/// * `limit` - Maximum emails per day
/// * `threshold_ratio` - Optional fraction of limit to enforce (e.g., 0.8 for 80%).
///   Use `None` or `Some(1.0)` for the full limit.
///
/// Returns error if threshold/limit reached. Useful for reserving capacity for priority emails.
#[allow(clippy::cast_possible_truncation)]
pub async fn try_acquire_email_slot(
    db_pool: &PgPool,
    limit: u32,
    threshold_ratio: Option<f64>,
) -> Result<()> {
    let effective_ratio = threshold_ratio.unwrap_or(1.0);
    log::debug!(
        "# | db::try_acquire_email_slot | limit={limit}, threshold_ratio={effective_ratio}"
    );

    let row = sqlx::query!(
        "INSERT INTO email_rate_limit (id, current_day, count)
        VALUES (1, CURRENT_DATE, 0)
        ON CONFLICT (id) DO UPDATE
        SET current_day = CURRENT_DATE
        WHERE email_rate_limit.id = 1
        RETURNING count, current_day",
    )
    .fetch_one(db_pool)
    .await
    .map_err(|e| anyhow!(e))?;

    if row.current_day != chrono::Utc::now().date_naive() {
        return Err(anyhow!("Email rate limit table has unexpected date"));
    }

    let threshold = (f64::from(limit) * effective_ratio) as i32;
    if row.count >= threshold {
        return Err(anyhow!(
            "Email rate limit {} exceeded: {} emails sent today, threshold is {} ({}% of {})",
            if effective_ratio < 1.0 {
                "threshold"
            } else {
                "limit"
            },
            row.count,
            threshold,
            (effective_ratio * 100.0) as i32,
            limit
        ));
    }

    sqlx::query!("UPDATE email_rate_limit SET count = count + 1 WHERE id = 1",)
        .execute(db_pool)
        .await
        .map_err(|e| anyhow!(e))?;

    Ok(())
}

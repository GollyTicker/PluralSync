use anyhow::{Result, anyhow};
use sqlx::PgPool;

pub async fn try_acquire_email_slot(db_pool: &PgPool, limit: u32) -> Result<()> {
    log::debug!("# | db::try_acquire_email_slot | limit={limit}");

    // First, check current count (after ensuring today's row exists)
    // This atomic operation ensures the row exists for today and returns current count
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

    log::debug!("# | db::try_acquire_email_slot | current count={row:?}");

    // If current_day is not today, something went wrong
    if row.current_day != chrono::Utc::now().date_naive() {
        return Err(anyhow!("Email rate limit table has unexpected date"));
    }

    // Check if we've already reached the limit
    if row.count >= limit as i32 {
        return Err(anyhow!(
            "Email rate limit exceeded: {} emails sent today, limit is {}",
            row.count,
            limit
        ));
    }

    // Increment the count (we know we're under the limit)
    sqlx::query!(
        "UPDATE email_rate_limit
        SET count = count + 1
        WHERE id = 1",
    )
    .execute(db_pool)
    .await
    .map_err(|e| anyhow!(e))?;

    Ok(())
}

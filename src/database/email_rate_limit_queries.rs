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
        SET current_day = CURRENT_DATE,
            count = CASE
                WHEN email_rate_limit.current_day != CURRENT_DATE THEN 0
                ELSE email_rate_limit.count
            END
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
            "EMAIL_RATE_LIMIT_THRESHOLD_EXCEEDED: {} emails sent today, threshold is {} ({}% of {})",
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

#[cfg(test)]
mod tests {
    use super::*;

    // === Test Constants ===
    const DEFAULT_THRESHOLD: f64 = 0.8;
    const FULL_THRESHOLD: f64 = 1.0;

    // === Helper Functions ===

    /// Reset the rate limit table for testing
    async fn reset_rate_limit_table(pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM email_rate_limit WHERE id = 1")
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Acquire multiple email slots sequentially
    async fn acquire_slots(
        pool: &PgPool,
        limit: u32,
        threshold: Option<f64>,
        count: u32,
    ) -> Result<(), anyhow::Error> {
        for i in 1..=count {
            try_acquire_email_slot(pool, limit, threshold)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to acquire slot {i}: {e}"))?;
        }
        Ok(())
    }

    /// Get the current email count from the database
    async fn get_current_count(pool: &PgPool) -> Result<i32, sqlx::Error> {
        sqlx::query_scalar("SELECT count FROM email_rate_limit WHERE id = 1")
            .fetch_one(pool)
            .await
    }

    // === Tests ===

    #[sqlx::test(migrations = "docker/migrations")]
    async fn test_try_acquire_email_slot_threshold_at_80_percent(
        pool: PgPool,
    ) -> Result<(), anyhow::Error> {
        // === Arrange ===
        reset_rate_limit_table(&pool).await?;
        let limit = 10u32;
        let threshold = DEFAULT_THRESHOLD;

        // === Act ===
        acquire_slots(&pool, limit, Some(threshold), 8).await?;
        let result = try_acquire_email_slot(&pool, limit, Some(threshold)).await;

        // === Assert ===
        assert!(
            result.is_err(),
            "Should fail when at threshold (8 out of 10)"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("EMAIL_RATE_LIMIT_THRESHOLD_EXCEEDED")
        );

        Ok(())
    }

    #[sqlx::test(migrations = "docker/migrations")]
    async fn test_try_acquire_email_slot_threshold_at_100_percent(
        pool: PgPool,
    ) -> Result<(), anyhow::Error> {
        // === Arrange ===
        reset_rate_limit_table(&pool).await?;
        let limit = 5u32;

        // === Act ===
        acquire_slots(&pool, limit, Some(FULL_THRESHOLD), 5).await?;
        let result = try_acquire_email_slot(&pool, limit, Some(FULL_THRESHOLD)).await;

        // === Assert ===
        assert!(result.is_err(), "Should fail when at limit (5 out of 5)");

        Ok(())
    }

    #[sqlx::test(migrations = "docker/migrations")]
    async fn test_try_acquire_email_slot_boundary_exactly_at_threshold(
        pool: PgPool,
    ) -> Result<(), anyhow::Error> {
        // === Arrange ===
        reset_rate_limit_table(&pool).await?;
        let limit = 10u32;
        let threshold = DEFAULT_THRESHOLD; // 80% of 10 = 8

        // === Act ===
        // Acquire exactly 7 slots (one below threshold)
        acquire_slots(&pool, limit, Some(threshold), 7).await?;

        // 8th should succeed (reaches threshold)
        try_acquire_email_slot(&pool, limit, Some(threshold))
            .await
            .expect("Should succeed when exactly at threshold - 1");

        // 9th should fail (at threshold)
        let result = try_acquire_email_slot(&pool, limit, Some(threshold)).await;

        // === Assert ===
        assert!(
            result.is_err(),
            "Should fail when count == threshold (8 out of 8)"
        );

        Ok(())
    }

    #[sqlx::test(migrations = "docker/migrations")]
    async fn test_try_acquire_email_slot_day_rollover(pool: PgPool) -> Result<(), anyhow::Error> {
        // === Arrange ===
        reset_rate_limit_table(&pool).await?;
        let limit = 5u32;

        // Acquire some slots today
        acquire_slots(&pool, limit, None, 3).await?;

        // Manually set the current_day to yesterday to simulate rollover
        sqlx::query!(
            "UPDATE email_rate_limit SET current_day = CURRENT_DATE - INTERVAL '1 day' WHERE id = 1"
        )
        .execute(&pool)
        .await?;

        // === Act ===
        // Try to acquire more - should reset and succeed
        try_acquire_email_slot(&pool, limit, None)
            .await
            .expect("Should succeed after day rollover (count should reset)");

        // === Assert ===
        let count = get_current_count(&pool).await?;
        assert_eq!(count, 1, "Count should be reset to 1 after day rollover");

        Ok(())
    }

    #[sqlx::test(migrations = "docker/migrations")]
    async fn test_try_acquire_email_slot_none_uses_full_limit(
        pool: PgPool,
    ) -> Result<(), anyhow::Error> {
        // === Arrange ===
        reset_rate_limit_table(&pool).await?;
        let limit = 3u32;

        // === Act ===
        acquire_slots(&pool, limit, None, 3).await?;
        let result = try_acquire_email_slot(&pool, limit, None).await;

        // === Assert ===
        assert!(result.is_err(), "Should fail when at limit");

        Ok(())
    }

    #[sqlx::test(migrations = "docker/migrations")]
    async fn test_try_acquire_email_slot_threshold_calculation_truncation(
        pool: PgPool,
    ) -> Result<(), anyhow::Error> {
        // === Arrange ===
        reset_rate_limit_table(&pool).await?;
        let limit = 7u32;
        let threshold = DEFAULT_THRESHOLD; // 80% of 7 = 5.6, truncated to 5

        // === Act ===
        acquire_slots(&pool, limit, Some(threshold), 5).await?;
        let result = try_acquire_email_slot(&pool, limit, Some(threshold)).await;

        // === Assert ===
        assert!(
            result.is_err(),
            "Should fail at 6 due to integer truncation (5.6 -> 5)"
        );

        Ok(())
    }
}

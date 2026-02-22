use anyhow::{Result, anyhow};
use sqlx::PgPool;

use crate::{history::HistoryEntry, users::UserId};

// ============================================================================
// History Queries
// ============================================================================

pub async fn insert_history_entry(
    db_pool: &PgPool,
    user_id: &UserId,
    status_text: &str,
) -> Result<()> {
    log::debug!("# | db::insert_history_entry | {user_id}");

    // Get the most recent entry to check for duplicates
    let recent_entries = get_history_entries(db_pool, user_id, 1).await?;
    if let Some(most_recent) = recent_entries.first()
        && most_recent.status_text == status_text
    {
        log::debug!("# | db::insert_history_entry | {user_id} | skipping duplicate entry");
        return Ok(());
    }

    sqlx::query!(
        "INSERT INTO history_status (user_id, status_text)
        VALUES ($1, $2)",
        user_id.inner,
        status_text
    )
    .execute(db_pool)
    .await
    .map(|_| ())
    .map_err(|e| anyhow!(e))
}

pub async fn get_history_entries(
    db_pool: &PgPool,
    user_id: &UserId,
    limit: usize,
) -> Result<Vec<HistoryEntry>> {
    let limit: i64 = limit.try_into()?;
    log::debug!("# | db::get_history_entries | {user_id} | limit={limit}");
    sqlx::query_as!(
        HistoryEntry,
        "SELECT
            id,
            user_id,
            status_text,
            created_at
        FROM history_status
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2",
        user_id.inner,
        limit
    )
    .fetch_all(db_pool)
    .await
    .map_err(|e| anyhow!(e))
}

pub async fn prune_history(
    db_pool: &PgPool,
    user_id: &UserId,
    history_limit: usize,
    history_truncate_after_days: usize,
) -> Result<()> {
    let history_limit: i64 = history_limit.try_into()?;
    log::debug!(
        "# | db::prune_history | {user_id} | limit={history_limit}, days={history_truncate_after_days}"
    );

    // Prune by count and/or age in a single query
    // If limit is 0, all entries are removed (disables history)
    // If days is 0, no age-based pruning occurs
    sqlx::query!(
        "DELETE FROM history_status
        WHERE user_id = $1
          AND (
            -- Prune by count: keep only the most recent N entries
            (id NOT IN (
              SELECT id FROM history_status
              WHERE user_id = $1
              ORDER BY created_at DESC
              LIMIT $2
            ))
            OR
            -- Prune by age: remove entries older than N days (0 disables)
            (created_at <= NOW() - ($3 || ' days')::INTERVAL)
          )",
        user_id.inner,
        history_limit,
        history_truncate_after_days.to_string()
    )
    .execute(db_pool)
    .await
    .map(|_| ())
    .map_err(|e| anyhow!(e))
}

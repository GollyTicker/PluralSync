use crate::users::UserId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, Type)]
pub struct HistoryEntry {
    pub id: String,
    pub user_id: UserId,
    pub status_text: String,
    #[specta(type = String)]
    pub created_at: DateTime<Utc>,
}

/// Store a new history entry and then prune old entries based on user's config limits.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `user_id` - The user ID
/// * `status_text` - The formatted status text to store
/// * `history_limit` - Maximum number of entries to keep (0 disables history)
/// * `history_truncate_after_days` - Days after which entries are pruned (0 disables age-based pruning)
pub async fn store_history_entry(
    pool: &sqlx::PgPool,
    user_id: &UserId,
    status_text: &str,
    history_limit: i32,
    history_truncate_after_days: i32,
) -> Result<(), anyhow::Error> {
    let user_id_clone = user_id.clone();
    let status_text_clone = status_text.to_string();

    // Insert the new entry
    sqlx::query(
        r"
        INSERT INTO history_status (user_id, status_text)
        VALUES ($1, $2)
        ",
    )
    .bind(&user_id_clone)
    .bind(&status_text_clone)
    .execute(pool)
    .await?;

    // Prune old entries
    prune_history(pool, user_id, history_limit, history_truncate_after_days).await?;

    Ok(())
}

/// Get the most recent history entries for a user.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `user_id` - The user ID
/// * `limit` - Maximum number of entries to return
pub async fn get_history_entries(
    pool: &sqlx::PgPool,
    user_id: &UserId,
    limit: i32,
) -> Result<Vec<HistoryEntry>, anyhow::Error> {
    let entries = sqlx::query_as::<_, HistoryEntry>(
        r"
        SELECT id, user_id, status_text, created_at
        FROM history_status
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2
        ",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(entries)
}

/// Get the most recent status text for deduplication.
pub async fn get_most_recent_status_text(
    pool: &sqlx::PgPool,
    user_id: &UserId,
) -> Result<Option<String>, anyhow::Error> {
    let result: Option<(String,)> = sqlx::query_as(
        r"
        SELECT status_text
        FROM history_status
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT 1
        ",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|(text,)| text))
}

/// Prune old history entries based on count and age limits.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `user_id` - The user ID
/// * `history_limit` - Maximum number of entries to keep (0 removes all)
/// * `history_truncate_after_days` - Days after which entries are pruned (0 disables age-based pruning)
async fn prune_history(
    pool: &sqlx::PgPool,
    user_id: &UserId,
    history_limit: i32,
    history_truncate_after_days: i32,
) -> Result<(), anyhow::Error> {
    // If limit is 0, delete all entries
    if history_limit == 0 {
        sqlx::query(
            r"
            DELETE FROM history_status
            WHERE user_id = $1
            ",
        )
        .bind(user_id)
        .execute(pool)
        .await?;
        return Ok(());
    }

    // Build the prune query based on the parameters
    let prune_query = if history_truncate_after_days > 0 {
        // Both count and age pruning
        r"
        DELETE FROM history_status
        WHERE user_id = $1
          AND (
            -- Prune by count: keep only the most recent N entries
            id NOT IN (
              SELECT id FROM history_status
              WHERE user_id = $1
              ORDER BY created_at DESC
              LIMIT $2
            )
            OR
            -- Prune by age: remove entries older than N days
            created_at < NOW() - ($3 || ' days')::INTERVAL
          )
        "
    } else {
        // Only count-based pruning
        r"
        DELETE FROM history_status
        WHERE user_id = $1
          AND id NOT IN (
            SELECT id FROM history_status
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2
          )
        "
    };

    sqlx::query(prune_query)
        .bind(user_id)
        .bind(history_limit)
        .bind(history_truncate_after_days)
        .execute(pool)
        .await?;

    Ok(())
}

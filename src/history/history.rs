use crate::database;
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
    // Insert the new entry
    database::insert_history_entry(pool, user_id, status_text).await?;

    // Prune old entries
    database::prune_history(pool, user_id, history_limit, history_truncate_after_days).await?;

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
    let entries = database::get_history_entries(pool, user_id, limit).await?;

    Ok(entries)
}

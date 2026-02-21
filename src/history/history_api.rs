use crate::database;
use crate::users::UserId;
use chrono::{DateTime, Utc};
use rocket::{State, get, serde::json::Json};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::{FromRow, PgPool};

use crate::meta_api::HttpResult;
use crate::meta_api::expose_internal_error;
use crate::users::Jwt;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, Type)]
pub struct HistoryEntry {
    pub id: String,
    pub user_id: UserId,
    pub status_text: String,
    #[specta(type = String)]
    pub created_at: DateTime<Utc>,
}

#[get("/api/user/history/fronting")]
pub async fn get_api_user_history_fronting(
    db_pool: &State<PgPool>,
    jwt: Jwt,
    client: &State<reqwest::Client>,
    app_user_secrets: &State<database::ApplicationUserSecrets>,
) -> HttpResult<Json<Vec<HistoryEntry>>> {
    let user_id = jwt.user_id().map_err(expose_internal_error)?;
    log::info!("# | GET /api/user/history/fronting | {user_id}");

    // Get user's config with validated history_limit
    let user_config =
        database::get_user_config_with_secrets(db_pool, &user_id, client.inner(), app_user_secrets)
            .await
            .map_err(expose_internal_error)?;

    let limit: usize = user_config.history_limit;
    let entries = database::get_history_entries(db_pool, &user_id, limit)
        .await
        .map_err(expose_internal_error)?;

    log::info!(
        "# | GET /api/user/history/fronting | {user_id} | retrieved {} entries",
        entries.len()
    );

    Ok(Json(entries))
}

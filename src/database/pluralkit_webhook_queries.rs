use sqlx::PgPool;

use crate::users::UserId;

/// Get all users with `PluralKit` webhook enabled
pub async fn get_users_with_pluralkit_webhook_enabled(pool: &PgPool) -> sqlx::Result<Vec<UserId>> {
    sqlx::query_as!(
        UserId,
        r#"SELECT
            id AS inner
        FROM users
        WHERE
            enable_from_pluralkit = true
            AND enc__pluralkit_token IS NOT NULL
        "#
    )
    .fetch_all(pool)
    .await
}
